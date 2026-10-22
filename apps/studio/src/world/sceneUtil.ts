import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { clone as cloneSkinned } from "three/examples/jsm/utils/SkeletonUtils.js";

/** Frees the GPU resources of everything under `root` (geometry, materials and their textures). */
export function disposeTree(root: THREE.Object3D) {
  root.traverse((obj) => {
    const mesh = obj as THREE.Mesh;
    mesh.geometry?.dispose();
    const materials = Array.isArray(mesh.material) ? mesh.material : mesh.material ? [mesh.material] : [];
    for (const material of materials) {
      for (const value of Object.values(material)) {
        if (value instanceof THREE.Texture) value.dispose();
      }
      material.dispose();
    }
    (obj as THREE.InstancedMesh).dispose?.();
  });
}

export function parseGlb(buffer: ArrayBuffer): Promise<THREE.Object3D> {
  return new Promise((resolve, reject) => {
    new GLTFLoader().parse(buffer, "", (gltf) => resolve(gltf.scene), reject);
  });
}

/** A copy of a parsed model that shares its geometry and materials (drawn double-sided, never culled when skinned). */
export function cloneModel(model: THREE.Object3D): THREE.Object3D {
  const instance = cloneSkinned(model);
  instance.traverse((obj) => {
    const mesh = obj as THREE.SkinnedMesh;
    if (mesh.isSkinnedMesh) mesh.frustumCulled = false;
    if (mesh.isMesh) {
      const material = (Array.isArray(mesh.material) ? mesh.material : [mesh.material]) as THREE.Material[];
      for (const m of material) m.side = THREE.DoubleSide;
    }
  });
  return instance;
}

/** Releases what only this copy owns (the skeletons); its geometry and materials belong to the model it was cloned from. */
export function disposeClone(instance: THREE.Object3D) {
  instance.traverse((obj) => (obj as THREE.SkinnedMesh).skeleton?.dispose());
}
