import * as THREE from "three";
import { t } from "../i18n";
import { MAP_SIZE, MODEL_UNITS_PER_TILE } from "./layout";
import { disposeTree, drawDoubleSided, parseGlb } from "./sceneUtil";
import { TransientRenderError } from "./thumbnails";
import type { ThumbnailRender } from "./thumbnails";

/** How often a lost WebGL context is replaced before pictures fail for good. */
const MAX_CONTEXTS = 3;

/** Half the picture's width in model units: a map is `MAP_SIZE` tiles wide, so it fills the picture. */
const HALF_WIDTH = (MAP_SIZE * MODEL_UNITS_PER_TILE) / 2;

/**
 * A renderer that draws a map's terrain model from straight above into a `width` x `height` picture (north up). The map's width fills
 * the picture; a picture wider than square (the preview's 256 x 192, 4 : 3) cuts as much off the top as off the bottom.
 * One `WebGLRenderer` on a detached canvas serves every call; it is created on the first one.
 * Calls must not overlap (drawing is synchronous once the model is parsed, and the cache runs few at a time).
 * A lost context would draw blank pictures: the call that meets one throws a `TransientRenderError` (the cache then
 * forgets the request) and the next call creates a new context, up to `MAX_CONTEXTS` in all.
 */
export function createTopDownRenderer(width = 64, height = width): ThumbnailRender {
  let gl: THREE.WebGLRenderer | null = null;
  let contexts = 0;
  const scene = new THREE.Scene();
  scene.add(new THREE.AmbientLight(0xffffff, 1.0));
  const sun = new THREE.DirectionalLight(0xffffff, 0.6);
  sun.position.set(0, 1, 0);
  scene.add(sun);
  const halfDepth = (HALF_WIDTH * height) / width;
  const camera = new THREE.OrthographicCamera(-HALF_WIDTH, HALF_WIDTH, halfDepth, -halfDepth);
  camera.up.set(0, 0, -1);

  return async (glb) => {
    const model = await parseGlb(glb);
    drawDoubleSided(model);
    try {
      if (!gl) {
        if (contexts >= MAX_CONTEXTS) throw new Error(t("world:thumbnails.contextLostForGood", { limit: MAX_CONTEXTS }));
        contexts++;
        gl = new THREE.WebGLRenderer({
          canvas: document.createElement("canvas"),
          antialias: true,
          preserveDrawingBuffer: true,
        });
        gl.setPixelRatio(1);
        gl.setSize(width, height, false);
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
      if (gl.getContext().isContextLost()) {
        gl.dispose();
        gl = null;
        throw new TransientRenderError(t("world:thumbnails.contextLost"));
      }
      return gl.domElement.toDataURL("image/png");
    } finally {
      scene.remove(model);
      disposeTree(model);
    }
  };
}
