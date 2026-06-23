import * as THREE from "three";
import type { ShamrockSceneState } from "./types";

type HpBar = Readonly<{
  fill: THREE.Mesh<THREE.PlaneGeometry, THREE.MeshBasicMaterial>;
  width: number;
  x: number;
}>;

export class ShamrockSceneRenderer {
  private readonly renderer: THREE.WebGLRenderer;
  private readonly scene = new THREE.Scene();
  private readonly camera = new THREE.OrthographicCamera(-8, 8, 5, -5, 0.1, 100);
  private readonly clock = new THREE.Clock();
  private readonly playerSprite: THREE.Sprite;
  private readonly opponentSprite: THREE.Sprite;
  private readonly playerBar: HpBar;
  private readonly opponentBar: HpBar;
  private readonly lightBand: THREE.Mesh<THREE.PlaneGeometry, THREE.MeshBasicMaterial>;
  private readonly centerGlint: THREE.Mesh<THREE.PlaneGeometry, THREE.MeshBasicMaterial>;
  private animationFrame: number | null = null;

  constructor(private readonly canvas: HTMLCanvasElement, initialState: ShamrockSceneState) {
    this.renderer = new THREE.WebGLRenderer({
      canvas,
      antialias: false,
      alpha: false,
      preserveDrawingBuffer: true,
      powerPreference: "high-performance",
    });
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    this.camera.position.set(0, 0, 10);
    this.scene.background = new THREE.Color("#bfe8fb");

    this.addStage();
    this.lightBand = this.addLightBand();
    this.centerGlint = this.addCenterGlint();
    this.playerSprite = this.addSprite("player");
    this.opponentSprite = this.addSprite("opponent");
    this.playerBar = this.addHpBar(-3.35, -2.88, initialState.playerHpRatio);
    this.opponentBar = this.addHpBar(3.12, 2.55, initialState.opponentHpRatio);
    this.resize();
  }

  start(): void {
    if (this.animationFrame !== null) return;
    const render = () => {
      const time = this.clock.getElapsedTime();
      this.playerSprite.position.y = -1.55 + Math.sin(time * 2.2) * 0.05;
      this.opponentSprite.position.y = 1.34 + Math.sin(time * 2.0 + 1.2) * 0.04;
      this.playerSprite.rotation.z = Math.sin(time * 1.4) * 0.015;
      this.opponentSprite.rotation.z = Math.sin(time * 1.35 + 0.7) * 0.012;
      this.lightBand.material.opacity = 0.08 + Math.sin(time * 2.6) * 0.045;
      this.centerGlint.material.color.set(Math.sin(time * 8) > 0 ? "#fff3a3" : "#f0b94d");
      this.renderer.render(this.scene, this.camera);
      this.animationFrame = window.requestAnimationFrame(render);
    };
    render();
  }

  update(nextState: ShamrockSceneState): void {
    this.setHpBar(this.playerBar, nextState.playerHpRatio);
    this.setHpBar(this.opponentBar, nextState.opponentHpRatio);
    this.playerSprite.material.opacity = nextState.selectedActionIndex === 0 ? 1 : 0.94;
  }

  resize(): void {
    const bounds = this.canvas.getBoundingClientRect();
    const width = Math.max(1, Math.floor(bounds.width));
    const height = Math.max(1, Math.floor(bounds.height));
    this.renderer.setSize(width, height, false);

    const viewHeight = 7.8;
    const viewWidth = viewHeight * (width / height);
    this.camera.left = -viewWidth / 2;
    this.camera.right = viewWidth / 2;
    this.camera.top = viewHeight / 2;
    this.camera.bottom = -viewHeight / 2;
    this.camera.updateProjectionMatrix();
  }

  dispose(): void {
    if (this.animationFrame !== null) {
      window.cancelAnimationFrame(this.animationFrame);
      this.animationFrame = null;
    }
    this.scene.traverse((object) => {
      if (object instanceof THREE.Mesh) {
        object.geometry.dispose();
        object.material.dispose();
      }
      if (object instanceof THREE.Sprite) {
        object.material.map?.dispose();
        object.material.dispose();
      }
    });
    this.renderer.dispose();
  }

  private addStage(): void {
    this.addPlane("#dff7b7", 0, -1.88, 18, 4.8);
    this.addPlane("#9ed7f4", 0, 2.22, 18, 3.7);
    this.addPlane("#6fbc73", 0, -0.15, 18, 0.55);
    this.addPlane("#5aa461", 0, -0.48, 18, 0.24);

    this.addPlatform(3.05, 1.08, 2.75, 0.64, "#8ac572", "#d7eb95");
    this.addPlatform(-3.05, -1.95, 3.25, 0.82, "#76b45e", "#c8df86");
    this.addShadow(3.05, 0.95, 1.75, 0.25);
    this.addShadow(-3.05, -2.06, 1.95, 0.3);

    for (let index = 0; index < 16; index += 1) {
      const x = -8 + index * 1.08;
      const y = -0.4 + Math.sin(index) * 0.08;
      this.addPlane(index % 2 === 0 ? "#7fbd69" : "#68ad5e", x, y, 0.45, 0.08);
    }
  }

  private addLightBand(): THREE.Mesh<THREE.PlaneGeometry, THREE.MeshBasicMaterial> {
    const band = new THREE.Mesh(
      new THREE.PlaneGeometry(18, 0.42),
      new THREE.MeshBasicMaterial({
        color: "#fff7b8",
        transparent: true,
        opacity: 0.08,
      }),
    );
    band.position.set(0, 0.54, -0.18);
    this.scene.add(band);
    return band;
  }

  private addCenterGlint(): THREE.Mesh<THREE.PlaneGeometry, THREE.MeshBasicMaterial> {
    const glint = new THREE.Mesh(
      new THREE.PlaneGeometry(0.34, 0.34),
      new THREE.MeshBasicMaterial({
        color: "#fff3a3",
      }),
    );
    glint.position.set(0, 0, 0.1);
    this.scene.add(glint);
    return glint;
  }

  private addPlane(color: string, x: number, y: number, width: number, height: number): void {
    const mesh = new THREE.Mesh(
      new THREE.PlaneGeometry(width, height),
      new THREE.MeshBasicMaterial({ color }),
    );
    mesh.position.set(x, y, -2);
    this.scene.add(mesh);
  }

  private addPlatform(
    x: number,
    y: number,
    width: number,
    height: number,
    rimColor: string,
    topColor: string,
  ): void {
    const rim = new THREE.Mesh(
      new THREE.CircleGeometry(1, 48),
      new THREE.MeshBasicMaterial({ color: rimColor }),
    );
    rim.scale.set(width, height, 1);
    rim.position.set(x, y, -0.75);
    this.scene.add(rim);

    const top = new THREE.Mesh(
      new THREE.CircleGeometry(1, 48),
      new THREE.MeshBasicMaterial({ color: topColor }),
    );
    top.scale.set(width * 0.86, height * 0.72, 1);
    top.position.set(x, y + height * 0.08, -0.65);
    this.scene.add(top);
  }

  private addShadow(x: number, y: number, width: number, height: number): void {
    const shadow = new THREE.Mesh(
      new THREE.CircleGeometry(1, 36),
      new THREE.MeshBasicMaterial({
        color: "#21381d",
        transparent: true,
        opacity: 0.22,
      }),
    );
    shadow.scale.set(width, height, 1);
    shadow.position.set(x, y, -0.3);
    this.scene.add(shadow);
  }

  private addSprite(kind: "player" | "opponent"): THREE.Sprite {
    const texture = makeCreatureTexture(kind);
    const sprite = new THREE.Sprite(
      new THREE.SpriteMaterial({
        map: texture,
        transparent: true,
        alphaTest: 0.08,
      }),
    );
    if (kind === "player") {
      sprite.scale.set(1.95, 1.95, 1);
      sprite.position.set(-3.05, -1.55, 0.3);
    } else {
      sprite.scale.set(1.62, 1.62, 1);
      sprite.position.set(3.05, 1.34, 0.3);
    }
    this.scene.add(sprite);
    return sprite;
  }

  private addHpBar(x: number, y: number, ratio: number): HpBar {
    const width = 1.85;
    const back = new THREE.Mesh(
      new THREE.PlaneGeometry(width + 0.14, 0.18),
      new THREE.MeshBasicMaterial({ color: "#2d3b3c" }),
    );
    back.position.set(x, y, 0.45);
    this.scene.add(back);

    const fill = new THREE.Mesh(
      new THREE.PlaneGeometry(width, 0.09),
      new THREE.MeshBasicMaterial({ color: "#4fd067" }),
    );
    fill.position.set(x, y, 0.55);
    this.scene.add(fill);
    const bar = { fill, width, x };
    this.setHpBar(bar, ratio);
    return bar;
  }

  private setHpBar(bar: HpBar, ratio: number): void {
    const safeRatio = Math.max(0, Math.min(1, ratio));
    bar.fill.scale.x = safeRatio;
    bar.fill.position.x = bar.x - (bar.width * (1 - safeRatio)) / 2;
    bar.fill.material.color.set(safeRatio < 0.25 ? "#de5a45" : safeRatio < 0.55 ? "#e2b541" : "#4fd067");
  }
}

function makeCreatureTexture(kind: "player" | "opponent"): THREE.CanvasTexture {
  const canvas = document.createElement("canvas");
  canvas.width = 64;
  canvas.height = 64;
  const context = canvas.getContext("2d");
  if (!context) throw new Error("2D canvas context is unavailable.");
  context.imageSmoothingEnabled = false;
  context.clearRect(0, 0, 64, 64);

  if (kind === "player") {
    drawBlock(context, "#245c3d", 19, 24, 28, 24);
    drawBlock(context, "#3f9958", 14, 18, 36, 18);
    drawBlock(context, "#7acc68", 20, 12, 24, 18);
    drawBlock(context, "#f2f0c9", 25, 28, 14, 10);
    drawBlock(context, "#24352b", 20, 42, 8, 8);
    drawBlock(context, "#24352b", 38, 42, 8, 8);
    drawBlock(context, "#d5627b", 30, 8, 8, 8);
  } else {
    drawBlock(context, "#8a2e2b", 20, 19, 26, 28);
    drawBlock(context, "#dd6d35", 16, 14, 34, 20);
    drawBlock(context, "#ffd064", 26, 8, 16, 18);
    drawBlock(context, "#fff1a8", 30, 2, 8, 9);
    drawBlock(context, "#2b2330", 22, 40, 8, 8);
    drawBlock(context, "#2b2330", 38, 40, 8, 8);
    drawBlock(context, "#f6c48d", 27, 25, 14, 10);
  }

  const texture = new THREE.CanvasTexture(canvas);
  texture.magFilter = THREE.NearestFilter;
  texture.minFilter = THREE.NearestFilter;
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

function drawBlock(
  context: CanvasRenderingContext2D,
  color: string,
  x: number,
  y: number,
  width: number,
  height: number,
): void {
  context.fillStyle = color;
  context.fillRect(x, y, width, height);
}
