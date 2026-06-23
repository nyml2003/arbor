import { createEffect, onCleanup, onMount } from "solid-js";
import { ShamrockSceneRenderer } from "./scene";
import type { ShamrockSceneState } from "./types";

export function ShamrockScene(props: { state: ShamrockSceneState }) {
  let canvas!: HTMLCanvasElement;
  let sceneRenderer: ShamrockSceneRenderer | null = null;

  onMount(() => {
    sceneRenderer = new ShamrockSceneRenderer(canvas, props.state);
    const resize = () => sceneRenderer?.resize();
    window.addEventListener("resize", resize);
    sceneRenderer.start();
    resize();

    onCleanup(() => {
      window.removeEventListener("resize", resize);
      sceneRenderer?.dispose();
      sceneRenderer = null;
    });
  });

  createEffect(() => {
    sceneRenderer?.update(props.state);
  });

  return (
    <canvas
      ref={canvas}
      data-testid="shamrock-canvas"
      aria-label="Shamrock battle scene"
    />
  );
}
