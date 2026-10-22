import * as THREE from "three";
import { MAP_SIZE, MODEL_UNITS_PER_TILE } from "./layout";
import { disposeTree, drawDoubleSided, parseGlb } from "./sceneUtil";
import type { ThumbnailRender } from "./thumbnails";

/** Half the picture's width in model units: a map is `MAP_SIZE` tiles wide, so it fills the picture. */
const HALF_WIDTH = (MAP_SIZE * MODEL_UNITS_PER_TILE) / 2;

/**
 * A renderer that draws a map's terrain model from straight above into a `size` x `size` picture (north up).
 * One `WebGLRenderer` on a detached canvas serves every call; it is created on the first one.
 * Calls must not overlap (drawing is synchronous once the model is parsed, and the cache runs few at a time).
 */
export function createTopDownRenderer(size = 64): ThumbnailRender {
  let gl: THREE.WebGLRenderer | null = null;
  const scene = new THREE.Scene();
  scene.add(new THREE.AmbientLight(0xffffff, 1.0));
  const sun = new THREE.DirectionalLight(0xffffff, 0.6);
  sun.position.set(0, 1, 0);
  scene.add(sun);
  const camera = new THREE.OrthographicCamera(-HALF_WIDTH, HALF_WIDTH, HALF_WIDTH, -HALF_WIDTH);
  camera.up.set(0, 0, -1);

  return async (glb) => {
    const model = await parseGlb(glb);
    drawDoubleSided(model);
    try {
      if (!gl) {
        gl = new THREE.WebGLRenderer({
          canvas: document.createElement("canvas"),
          antialias: true,
          preserveDrawingBuffer: true,
        });
        gl.setPixelRatio(1);
        gl.setSize(size, size, false);
        gl.setClearColor(0x1b1b1f, 1);
      }
      // The 3D view places a map model with its origin on the map centre (scene/content.ts `addModel`), so look
      // at the origin: a model that does not span the whole map then sits in the picture as it does in the view.
      const box = new THREE.Box3().setFromObject(model);
      camera.position.set(0, box.max.y + 100, 0);
      camera.near = 1;
      camera.far = box.max.y - box.min.y + 200;
      camera.lookAt(0, 0, 0);
      camera.updateProjectionMatrix();
      scene.add(model);
      gl.render(scene, camera);
      return gl.domElement.toDataURL("image/png");
    } finally {
      scene.remove(model);
      disposeTree(model);
    }
  };
}
