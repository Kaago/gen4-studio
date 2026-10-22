import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import type { EventsDto, MatrixDto, TileBehavior } from "../../api";
import type { BuildingLayer, BuildingRef } from "../buildingLayer";
import type { EventRef } from "../events";
import { disposeMarkers } from "../markers";
import type { MarkerSet } from "../markers";
import { disposeTree } from "../sceneUtil";
import type { Loaded } from "./types";

/**
 * The three.js scene of the matrix view: renderer, camera, controls and the lights live as long as
 * the host; what the loaders put into it (content, markers, buildings, the maps behind them) is
 * plain state on the host that they replace as matrices come and go. The host renders on demand
 * (`render`, or `queueRender` for bursts) and frees all of it in `dispose`.
 */
export class SceneHost {
  readonly renderer: THREE.WebGLRenderer;
  readonly scene = new THREE.Scene();
  readonly camera: THREE.PerspectiveCamera;
  readonly controls: OrbitControls;
  private readonly observer: ResizeObserver;

  content = new THREE.Group();
  overlay: THREE.Object3D | null = null;
  /** Matrix and loaded maps behind the current content (null until a matrix is loaded). */
  matrix: MatrixDto | null = null;
  /** The terrain model instances in `content` (one per matrix cell showing a map), for reloading them alone. */
  models: THREE.Object3D[] = [];
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
  /** The buildings of the shown matrix (a child of `content`; null until a matrix is loaded). */
  buildings: BuildingLayer | null = null;
  /** The selected building, as the scene knows it (follows the `selectedMap`/`selectedBuilding` props). */
  selection: BuildingRef | null = null;
  /** Re-resolves the tile under the pointer on the next frame (set by the setup effect). */
  refreshHover: (() => void) | null = null;
  /** A render is already queued for the next animation frame. */
  renderQueued = false;
  /** The component is gone; late results and queued renders are dropped. */
  disposed = false;

  constructor(el: HTMLElement) {
    this.renderer = new THREE.WebGLRenderer({ antialias: true });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    this.renderer.setClearColor("#1e2430");
    el.appendChild(this.renderer.domElement);
    this.renderer.domElement.style.display = "block";

    this.scene.add(new THREE.HemisphereLight("#ffffff", "#667788", 1.6));
    const sun = new THREE.DirectionalLight("#ffffff", 1.5);
    sun.position.set(-40, 100, 60);
    this.scene.add(sun);
    this.camera = new THREE.PerspectiveCamera(50, 1, 0.5, 8000);
    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.scene.add(this.content);

    const resize = () => {
      const w = Math.max(el.clientWidth, 1);
      const h = Math.max(el.clientHeight, 1);
      this.renderer.setSize(w, h);
      this.camera.aspect = w / h;
      this.camera.updateProjectionMatrix();
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
   * the resize observer and the controls, the content (terrain models, overlay), the renderer.
   */
  dispose() {
    this.disposed = true;
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
