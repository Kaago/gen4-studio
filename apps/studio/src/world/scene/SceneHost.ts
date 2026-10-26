import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { effectiveTheme } from "../../ui/tokens/theme";
import type { EventsDto, MatrixDto, TileBehavior } from "../../api";
import type { CellRect } from "../../workbench/sceneScope";
import { sceneBackground } from "../colors";
import type { BuildingLayer, BuildingRef } from "../buildingLayer";
import type { EventRef } from "../events";
import { disposeMarkers } from "../markers";
import type { MarkerSet } from "../markers";
import { disposeTree } from "../sceneUtil";
import { focusHeight, topDownFrame } from "./camera";
import type { Loaded } from "./types";

/** The top-down camera stands this far above the height it looks at; its depth range reaches far below that height. */
const TOP_DOWN_HEIGHT = 2000;
const TOP_DOWN_FAR = 8000;

/**
 * The three.js scene of the matrix view: renderer, cameras, controls and the lights live as long as
 * the host (the top-down view swaps in an orthographic camera and controls of its own); what the
 * loaders put into it (content, markers, buildings, the maps behind them) is plain state on the host
 * that they replace as matrices come and go. The host renders on demand (`render`, or `queueRender`
 * for bursts) and frees all of it in `dispose`.
 */
export class SceneHost {
  readonly renderer: THREE.WebGLRenderer;
  readonly scene = new THREE.Scene();
  /** The camera the scene is drawn with: `slanted`, or an orthographic camera looking straight down while the top-down view is on. */
  camera: THREE.PerspectiveCamera | THREE.OrthographicCamera;
  /** The controls of `camera` (made anew with every camera; they never turn the top-down camera). */
  controls: OrbitControls;
  /** The perspective camera of the slanted view; while the top-down view is on it keeps its place, and `slantedTarget` its target. */
  readonly slanted: THREE.PerspectiveCamera;
  /** Where the slanted camera looks while the top-down view is on (the controls of that view have their own target). */
  readonly slantedTarget = new THREE.Vector3();
  /** The cells the top-down view fits in (null while it is off); a change of the panel's size fits them anew. */
  private topDownFocus: CellRect | null = null;
  /** The panel's width by its height. */
  private aspect = 1;
  private readonly observer: ResizeObserver;

  content = new THREE.Group();
  overlay: THREE.Object3D | null = null;
  /** Matrix and loaded maps behind the current content (null until a matrix is loaded). */
  matrix: MatrixDto | null = null;
  /** The terrain model instances in `content` (one per matrix cell showing a map), for reloading them alone. */
  models: THREE.Object3D[] = [];
  /**
   * Map id to its terrain model generation, bumped by every one-map model reload: a model fetched
   * under an older generation (by that reload or by a full model load) is dropped, never shown.
   */
  mapModelGen = new Map<number, number>();
  /** What the last full load could not read (names, maps), kept for the summary after a models-only reload. */
  loadProblems: { names: string | null; maps: string[] } = { names: null, maps: [] };
  loaded = new Map<number, Loaded>();
  tiles: THREE.InstancedMesh | null = null;
  /** Map id to the first overlay instance of each matrix cell showing that map. */
  bases = new Map<number, number[]>();
  /** Tile behaviour names by kind byte (empty until loaded). */
  names = new Map<number, TileBehavior>();
  markers: MarkerSet | null = null;
  /** The events the markers show (null: none shown, or they could not be read). */
  events: EventsDto | null = null;
  /** The selected event, as the scene knows it (follows the `selectedEvent` prop). */
  selectedEvent: EventRef | null = null;
  /** The events an error finding of the checks hits (follows the `faultyEvents` prop); their markers show the error colour. */
  faulty: readonly EventRef[] = [];
  /** The buildings of the shown matrix (a child of `content`; null until a matrix is loaded). */
  buildings: BuildingLayer | null = null;
  /** The selected building, as the scene knows it (follows the `selectedMap`/`selectedBuilding` props). */
  selection: BuildingRef | null = null;
  /** The selected matrix cell, as the scene knows it (follows the `selectedCell` prop); `cellFrame` outlines it. */
  selectedCell: { x: number; y: number } | null = null;
  /** Holds the frame around the selected cell (see `showCellFrame`); it outlives the content of a matrix. */
  readonly cellFrame = new THREE.Group();
  private frame: THREE.LineLoop | null = null;
  /** Re-resolves the tile under the pointer on the next frame (set by the setup effect). */
  refreshHover: (() => void) | null = null;
  /** A render is already queued for the next animation frame. */
  renderQueued = false;
  /** The component is gone; late results and queued renders are dropped. */
  disposed = false;

  constructor(el: HTMLElement) {
    this.renderer = new THREE.WebGLRenderer({ antialias: true });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    this.renderer.setClearColor(sceneBackground(effectiveTheme()));
    el.appendChild(this.renderer.domElement);
    this.renderer.domElement.style.display = "block";

    // Lighting, not design colours: neutral white sun and a cool-grey ground bounce (as in thumbnailRenderer).
    this.scene.add(new THREE.HemisphereLight(0xffffff, 0x667788, 1.6));
    const sun = new THREE.DirectionalLight(0xffffff, 1.5);
    sun.position.set(-40, 100, 60);
    this.scene.add(sun);
    this.slanted = new THREE.PerspectiveCamera(50, 1, 0.5, 8000);
    this.camera = this.slanted;
    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.scene.add(this.content);
    this.scene.add(this.cellFrame);

    const resize = () => {
      const w = Math.max(el.clientWidth, 1);
      const h = Math.max(el.clientHeight, 1);
      this.renderer.setSize(w, h);
      this.aspect = w / h;
      this.slanted.aspect = this.aspect;
      this.slanted.updateProjectionMatrix();
      if (this.camera instanceof THREE.OrthographicCamera && this.topDownFocus) this.fitTopDown(this.camera, this.topDownFocus);
      this.render();
    };
    this.observer = new ResizeObserver(resize);
    this.observer.observe(el);
    this.controls.addEventListener("change", this.render);
    resize();
  }

  /** The element the scene is drawn on (pointer events are heard here). */
  get canvas(): HTMLCanvasElement {
    return this.renderer.domElement;
  }

  render = () => this.renderer.render(this.scene, this.camera);

  /** The top-down view is on. */
  get topDown(): boolean {
    return this.camera !== this.slanted;
  }

  /**
   * Switches the top-down view on or off. On: an orthographic camera looks straight down (north up) and frames `focus` of `matrix`; its
   * controls pan and zoom but never turn it. Off: the slanted camera comes back as it was left. Nothing happens when the view already is
   * as asked. Renders.
   */
  setTopDown(on: boolean, focus: CellRect, matrix: MatrixDto): void {
    if (on === this.topDown) return;
    if (on) {
      this.slantedTarget.copy(this.controls.target);
      const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.5, TOP_DOWN_FAR);
      camera.up.set(0, 0, -1);
      this.useCamera(camera);
      this.frameTopDown(focus, matrix);
    } else {
      this.topDownFocus = null;
      this.useCamera(this.slanted);
      this.controls.target.copy(this.slantedTarget);
      this.controls.update();
      this.render();
    }
  }

  /** Frames `focus` of `matrix` in the top-down view (zoom back to 1), looking at the height of its middle cell. Only while the view is on. Renders. */
  frameTopDown(focus: CellRect, matrix: MatrixDto): void {
    const camera = this.camera;
    if (!(camera instanceof THREE.OrthographicCamera)) return;
    this.topDownFocus = focus;
    camera.zoom = 1;
    const { target } = this.fitTopDown(camera, focus);
    target.y = focusHeight(focus, matrix);
    this.controls.target.copy(target);
    camera.position.set(target.x, target.y + TOP_DOWN_HEIGHT, target.z);
    this.controls.update();
    this.render();
  }

  /** Sets what `camera` shows so that `focus` fits in the panel's shape; returns the frame. */
  private fitTopDown(camera: THREE.OrthographicCamera, focus: CellRect) {
    const frame = topDownFrame(focus, this.aspect);
    camera.left = frame.left;
    camera.right = frame.right;
    camera.top = frame.top;
    camera.bottom = frame.bottom;
    camera.updateProjectionMatrix();
    return frame;
  }

  /** Draws with `camera` from now on, with new controls for it (the old ones are freed). */
  private useCamera(camera: THREE.PerspectiveCamera | THREE.OrthographicCamera) {
    this.controls.removeEventListener("change", this.render);
    this.controls.dispose();
    this.camera = camera;
    this.controls = new OrbitControls(camera, this.renderer.domElement);
    this.controls.enableRotate = camera === this.slanted;
    this.controls.addEventListener("change", this.render);
  }

  /** Sets the colour the scene is cleared with (the theme's world surface). It does not render, and touches neither models nor content. */
  setBackground(hex: string) {
    this.renderer.setClearColor(hex);
  }

  /** Puts `frame` around the selected cell (null: none) in place of the frame before it, which is freed. It does not render. */
  setCellFrame(frame: THREE.LineLoop | null) {
    if (this.frame) {
      this.cellFrame.remove(this.frame);
      this.frame.geometry.dispose();
      (this.frame.material as THREE.Material).dispose();
    }
    this.frame = frame;
    if (frame) this.cellFrame.add(frame);
  }

  /** Renders on the next animation frame; bursts of arriving models cost one render. */
  queueRender() {
    if (this.renderQueued) return;
    this.renderQueued = true;
    requestAnimationFrame(() => {
      this.renderQueued = false;
      if (!this.disposed) this.render();
    });
  }

  /**
   * Frees everything the scene holds, in this order: the markers, the buildings (their templates),
   * the resize observer and the controls, the content (terrain models, overlay), the renderer. The cell frame goes first.
   */
  dispose() {
    this.disposed = true;
    this.setCellFrame(null);
    if (this.markers) disposeMarkers(this.markers);
    this.buildings?.dispose();
    this.observer.disconnect();
    this.controls.removeEventListener("change", this.render);
    this.controls.dispose();
    disposeTree(this.content);
    this.renderer.dispose();
    this.renderer.domElement.remove();
  }
}
