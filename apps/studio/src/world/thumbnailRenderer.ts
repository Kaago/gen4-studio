import * as THREE from "three";
import { MAP_SIZE, MODEL_UNITS_PER_TILE } from "./layout";
import { disposeTree, parseGlb } from "./sceneUtil";
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
      // Frame the middle of the model whatever its origin, so every map fills the picture the same way.
      const box = new THREE.Box3().setFromObject(model);
      const centre = box.getCenter(new THREE.Vector3());
      camera.position.set(centre.x, box.max.y + 100, centre.z);
      camera.near = 1;
      camera.far = box.max.y - box.min.y + 200;
      camera.lookAt(centre.x, 0, centre.z);
      camera.updateProjectionMatrix();
      sun.target.position.set(centre.x, 0, centre.z);
      scene.add(sun.target, model);
      gl.render(scene, camera);
      return gl.domElement.toDataURL("image/png");
    } finally {
      scene.remove(model, sun.target);
      disposeTree(model);
    }
  };
}
