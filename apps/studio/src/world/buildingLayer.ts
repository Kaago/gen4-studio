import * as THREE from "three";
import type { BuildingDto, MapDto, MatrixDto } from "../api";
import { buildingFallbackBox, buildingKey, buildingModelIds, buildingTransform } from "./buildings";
import { cellOrigin, cellY } from "./layout";
import { cloneModel, disposeClone, disposeTree } from "./sceneUtil";

export interface BuildingRef {
  map: number;
  index: number;
}

/** One building as it stands in one matrix cell (`cell`: its scene origin and height) (a map shown in several cells has one slot per cell). */
interface Slot {
  ref: BuildingRef;
  cell: { x: number; y: number; z: number };
  building: BuildingDto;
  group: THREE.Group;
  box: THREE.Mesh;
  model: THREE.Object3D | null;
}

const boxGeometry = new THREE.BoxGeometry(1, 1, 1);
const boxMaterial = new THREE.MeshBasicMaterial({ color: "#f59e0b", wireframe: true });
const OUTLINE_COLOR = "#ffffff";

/**
 * The buildings of the shown matrix: one group per building and cell, holding the textured model
 * or, while it has none (not loaded yet, or it failed), a wireframe box. Models are parsed once
 * per model id and cloned per building. Selection draws an outline; `pick` raycasts the buildings.
 */
export class BuildingLayer {
  readonly root = new THREE.Group();
  private readonly slotGroup = new THREE.Group();
  private readonly outlineGroup = new THREE.Group();
  private slots = new Map<string, Slot[]>();
  private templates = new Map<number, THREE.Object3D>();
  private failures = new Map<number, string>();
  private selected: BuildingRef | null = null;

  constructor() {
    this.root.add(this.slotGroup, this.outlineGroup);
  }

  /** Rebuilds the slots of map `mapId` in every cell of `matrix` that shows it from `buildings`. */
  syncMap(matrix: MatrixDto, mapId: number, buildings: readonly BuildingDto[]) {
    for (const [key, list] of this.slots) {
      if (!key.startsWith(`${mapId}:`)) continue;
      for (const slot of list) this.removeSlot(slot);
      this.slots.delete(key);
    }
    matrix.maps.forEach((id, i) => {
      if (id !== mapId) return;
      const col = i % matrix.width;
      const row = Math.floor(i / matrix.width);
      const cell = { ...cellOrigin(col, row), y: cellY(matrix, col, row) };
      buildings.forEach((building, index) => this.addSlot({ map: mapId, index }, cell, building));
    });
    // The outline follows the rebuilt slots; a selection that no longer exists is dropped by the caller.
    this.select(this.selected);
  }

  /** Builds the slots of every loaded map (call once on an empty layer). */
  build(matrix: MatrixDto, loaded: ReadonlyMap<number, { map: MapDto }>) {
    for (const [id, data] of loaded) this.syncMap(matrix, id, data.map.buildings);
  }

  /** Distinct model ids of all buildings that have neither a model nor a recorded failure. */
  missingModelIds(): number[] {
    const all = [...this.slots.values()].flatMap((list) => list.map((s) => s.building));
    return buildingModelIds(all).filter((id) => !this.templates.has(id) && !this.failures.has(id));
  }

  /** Hands a parsed model to every building with that id (they clone it). */
  setTemplate(modelId: number, template: THREE.Object3D) {
    this.failures.delete(modelId);
    const old = this.templates.get(modelId);
    if (old) {
      this.detachModel(modelId);
      disposeTree(old);
    }
    this.templates.set(modelId, template);
    for (const list of this.slots.values()) {
      for (const slot of list) if (slot.building.model_id === modelId) this.attachModel(slot);
    }
    this.refreshOutlines();
  }

  /** Records why model `modelId` could not be loaded; its buildings keep their box. */
  setFailed(modelId: number, message: string) {
    this.failures.set(modelId, message);
  }

  /** Forgets that `modelId` failed, so `missingModelIds` offers it again. */
  retry(modelId: number) {
    this.failures.delete(modelId);
  }

  /** Why model `modelId` has no model, if it failed. */
  failure(modelId: number): string | undefined {
    return this.failures.get(modelId);
  }

  /** Drops every model (the textures changed): all buildings show their box until models are set again. */
  resetModels() {
    for (const id of [...this.templates.keys()]) {
      this.detachModel(id);
      const template = this.templates.get(id);
      if (template) disposeTree(template);
    }
    this.templates.clear();
    this.failures.clear();
    this.refreshOutlines();
  }

  /** The building under a ray, nearest first, or null. */
  pick(raycaster: THREE.Raycaster): { ref: BuildingRef; cell: { x: number; y: number; z: number }; building: BuildingDto } | null {
    for (const hit of raycaster.intersectObjects(this.slotGroup.children, true)) {
      let obj: THREE.Object3D | null = hit.object;
      while (obj && !obj.userData.slot) obj = obj.parent;
      const slot = obj?.userData.slot as Slot | undefined;
      if (slot) return { ref: slot.ref, cell: slot.cell, building: slot.building };
    }
    return null;
  }

  /** Marks building `ref` (null: none) with an outline. */
  select(ref: BuildingRef | null) {
    this.selected = ref;
    for (const helper of [...this.outlineGroup.children]) {
      this.outlineGroup.remove(helper);
      (helper as THREE.BoxHelper).geometry.dispose();
      ((helper as THREE.BoxHelper).material as THREE.Material).dispose();
    }
    if (!ref) return;
    for (const slot of this.slots.get(buildingKey(ref.map, ref.index)) ?? []) {
      const helper = new THREE.BoxHelper(slot.group, OUTLINE_COLOR);
      const material = helper.material as THREE.LineBasicMaterial;
      material.depthTest = false;
      helper.renderOrder = 12;
      helper.frustumCulled = false;
      this.outlineGroup.add(helper);
    }
  }

  /** Whether building `ref` exists in the scene. */
  has(ref: BuildingRef): boolean {
    return this.slots.has(buildingKey(ref.map, ref.index));
  }

  /** Shows building `ref` as `building` (a drag in progress, or the committed values) in all its cells. */
  show(ref: BuildingRef, building: BuildingDto) {
    for (const slot of this.slots.get(buildingKey(ref.map, ref.index)) ?? []) {
      const modelChanged = slot.building.model_id !== building.model_id;
      slot.building = building;
      if (modelChanged) this.attachModel(slot);
      else this.pose(slot);
    }
    this.refreshOutlines();
  }

  dispose() {
    for (const template of this.templates.values()) disposeTree(template);
    this.templates.clear();
    this.select(null);
    for (const list of this.slots.values()) for (const slot of list) this.removeSlot(slot);
    this.slots.clear();
  }

  private addSlot(ref: BuildingRef, cell: { x: number; y: number; z: number }, building: BuildingDto) {
    const box = new THREE.Mesh(boxGeometry, boxMaterial);
    const group = new THREE.Group();
    group.add(box);
    const slot: Slot = { ref, cell, building, group, box, model: null };
    group.userData.slot = slot;
    this.pose(slot);
    this.attachModel(slot);
    this.slotGroup.add(group);
    const key = buildingKey(ref.map, ref.index);
    const list = this.slots.get(key) ?? [];
    list.push(slot);
    this.slots.set(key, list);
  }

  private removeSlot(slot: Slot) {
    this.slotGroup.remove(slot.group);
    if (slot.model) disposeClone(slot.model);
  }

  /** Position, box size and model scale of a slot from its building. */
  private pose(slot: Slot) {
    const { position, scale } = buildingTransform(slot.building, slot.cell);
    slot.group.position.set(...position);
    const box = buildingFallbackBox(slot.building);
    slot.box.scale.set(...box.size);
    slot.box.position.set(0, box.offsetY, 0);
    slot.model?.scale.set(...scale);
  }

  private attachModel(slot: Slot) {
    if (slot.model) {
      slot.group.remove(slot.model);
      disposeClone(slot.model);
      slot.model = null;
    }
    const template = this.templates.get(slot.building.model_id);
    if (!template) {
      slot.group.add(slot.box);
      return;
    }
    // The box leaves the group: the outline is fitted to what the group contains.
    slot.group.remove(slot.box);
    slot.model = cloneModel(template);
    slot.group.add(slot.model);
    this.pose(slot);
  }

  /** Takes the model clones of `modelId` out of their buildings (which show their box again). */
  private detachModel(modelId: number) {
    for (const list of this.slots.values()) {
      for (const slot of list) {
        if (slot.building.model_id !== modelId || !slot.model) continue;
        slot.group.remove(slot.model);
        disposeClone(slot.model);
        slot.model = null;
        slot.group.add(slot.box);
      }
    }
  }

  private refreshOutlines() {
    for (const helper of this.outlineGroup.children) (helper as THREE.BoxHelper).update();
  }
}
