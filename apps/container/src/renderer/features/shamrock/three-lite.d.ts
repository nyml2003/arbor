declare module "three" {
  export const SRGBColorSpace: string;
  export const NearestFilter: number;

  export class Color {
    constructor(color: string);
    set(color: string): void;
  }

  export class Vector3 {
    x: number;
    y: number;
    z: number;
    set(x: number, y: number, z: number): void;
  }

  export class Euler {
    z: number;
  }

  export class Object3D {
    position: Vector3;
    rotation: Euler;
    scale: Vector3;
  }

  export class Scene extends Object3D {
    background: Color | null;
    add(object: Object3D): void;
    traverse(callback: (object: Object3D) => void): void;
  }

  export class OrthographicCamera extends Object3D {
    left: number;
    right: number;
    top: number;
    bottom: number;
    constructor(left: number, right: number, top: number, bottom: number, near: number, far: number);
    updateProjectionMatrix(): void;
  }

  export class Clock {
    getElapsedTime(): number;
  }

  export class WebGLRenderer {
    outputColorSpace: string;
    constructor(options: {
      canvas: HTMLCanvasElement;
      antialias?: boolean;
      alpha?: boolean;
      preserveDrawingBuffer?: boolean;
      powerPreference?: WebGLPowerPreference;
    });
    setPixelRatio(value: number): void;
    setSize(width: number, height: number, updateStyle: boolean): void;
    render(scene: Scene, camera: OrthographicCamera): void;
    dispose(): void;
  }

  export class PlaneGeometry {
    constructor(width: number, height: number);
    dispose(): void;
  }

  export class CircleGeometry {
    constructor(radius: number, segments: number);
    dispose(): void;
  }

export class MeshBasicMaterial {
    color: Color;
    opacity: number;
    constructor(options: { color: string; transparent?: boolean; opacity?: number });
    dispose(): void;
  }

  export class Mesh<
    G extends PlaneGeometry | CircleGeometry = PlaneGeometry | CircleGeometry,
    M extends MeshBasicMaterial = MeshBasicMaterial,
  > extends Object3D {
    geometry: G;
    material: M;
    constructor(geometry: G, material: M);
  }

  export class CanvasTexture {
    magFilter: number;
    minFilter: number;
    colorSpace: string;
    constructor(canvas: HTMLCanvasElement);
    dispose(): void;
  }

  export class SpriteMaterial {
    map?: CanvasTexture;
    opacity: number;
    constructor(options: { map: CanvasTexture; transparent?: boolean; alphaTest?: number });
    dispose(): void;
  }

  export class Sprite extends Object3D {
    material: SpriteMaterial;
    constructor(material: SpriteMaterial);
  }
}
