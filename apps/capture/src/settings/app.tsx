import { createSignal, onMount } from "solid-js";
import {
  type AreaSelection,
  type CaptureResult,
  defaultCaptureSettings,
  type CaptureSettings,
} from "../shared/contracts";
import {
  beginAreaCapture,
  getLastCaptureResult,
  getSettings,
  getLastSelection,
  openLastCapture,
  updateSettings,
} from "../shared/api";

function toErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return "发生了未知错误。";
}

export function App() {
  const [settings, setSettings] = createSignal<CaptureSettings>(
    defaultCaptureSettings,
  );
  const [lastSelection, setLastSelection] = createSignal<AreaSelection | null>(
    null,
  );
  const [lastCaptureResult, setLastCaptureResult] = createSignal<CaptureResult | null>(
    null,
  );
  const [status, setStatus] = createSignal("正在读取设置...");
  const [saving, setSaving] = createSignal(false);

  const refreshLastSelection = async () => {
    try {
      const next = await getLastSelection();
      setLastSelection(next);
    } catch {
      // Keep the settings surface usable even if the shell command is not ready.
    }
  };

  const refreshLastCaptureResult = async () => {
    try {
      const next = await getLastCaptureResult();
      setLastCaptureResult(next);
    } catch {
      // Keep the settings surface usable even if the shell command is not ready.
    }
  };

  onMount(() => {
    void (async () => {
      try {
        const next = await getSettings();
        setSettings(next);
        await refreshLastSelection();
        await refreshLastCaptureResult();
        setStatus("当前是空壳。设置读写已经接通，截图逻辑还没开始。");
      } catch (error: unknown) {
        setStatus(`读取设置失败：${toErrorMessage(error)}`);
      }
    })();

    const handleFocus = () => {
      void refreshLastSelection();
      void refreshLastCaptureResult();
    };

    window.addEventListener("focus", handleFocus);
    return () => {
      window.removeEventListener("focus", handleFocus);
    };
  });

  const updateField = <K extends keyof CaptureSettings>(
    key: K,
    value: CaptureSettings[K],
  ) => {
    setSettings((current) => ({
      ...current,
      [key]: value,
    }));
  };

  const handleSave = async (event: Event) => {
    event.preventDefault();
    setSaving(true);

    try {
      const next = await updateSettings(settings());
      setSettings(next);
      setStatus("设置已保存。");
    } catch (error: unknown) {
      setStatus(`保存失败：${toErrorMessage(error)}`);
    } finally {
      setSaving(false);
    }
  };

  const handleOverlayPreview = async () => {
    try {
      await beginAreaCapture();
      setStatus("已拉起 overlay。完成框选后，settings 页会显示最近一次选区。");
    } catch (error: unknown) {
      setStatus(`无法打开 overlay：${toErrorMessage(error)}`);
    }
  };

  const handleOpenLastCapture = async () => {
    try {
      await openLastCapture();
      setStatus("已请求打开最近一次截图。当前会调用系统默认图片查看器打开缓存文件。");
    } catch (error: unknown) {
      setStatus(`无法打开最近截图：${toErrorMessage(error)}`);
    }
  };

  return (
    <main class="page">
      <section class="panel">
        <header class="header">
          <p class="eyebrow">Capture</p>
          <h1 class="title">欢迎使用 Capture</h1>
          <p class="subtitle">
            现在已经可以做 Windows MVP 试用。最短路径是：按快捷键或点下面的按钮，框选，松开，直接去别的地方 `Ctrl+V`。
          </p>
        </header>

        <section class="welcomeCard">
          <div class="welcomeSteps">
            <div class="welcomeStep">
              <span class="stepIndex">1</span>
              <div>
                <h2 class="stepTitle">开始截图</h2>
                <p class="stepText">按 <strong>{settings().hotkey}</strong>，或者点下面的“开始区域截图”。</p>
              </div>
            </div>
            <div class="welcomeStep">
              <span class="stepIndex">2</span>
              <div>
                <h2 class="stepTitle">框选区域</h2>
                <p class="stepText">拖出一个矩形，松开鼠标。截图会自动保存到缓存，并写进剪贴板。</p>
              </div>
            </div>
            <div class="welcomeStep">
              <span class="stepIndex">3</span>
              <div>
                <h2 class="stepTitle">直接粘贴</h2>
                <p class="stepText">去聊天、文档或画图里按 <strong>Ctrl+V</strong>。右下角消息点一下会打开图片文件。</p>
              </div>
            </div>
          </div>

          <div class="welcomeActions">
            <button
              class="button buttonPrimary"
              type="button"
              onClick={() => {
                void handleOverlayPreview();
              }}
            >
              开始区域截图
            </button>
            <button
              class="button"
              type="button"
              onClick={() => {
                void handleOpenLastCapture();
              }}
            >
              打开最近截图
            </button>
          </div>
        </section>

        <form
          class="form"
          onSubmit={(event) => {
            void handleSave(event);
          }}
        >
          <label class="field">
            <span class="label">截图快捷键</span>
            <input
              class="input"
              type="text"
              value={settings().hotkey}
              onInput={(event) => {
                updateField("hotkey", event.currentTarget.value);
              }}
            />
          </label>

          <label class="field checkboxField">
            <input
              class="checkbox"
              type="checkbox"
              checked={settings().notification_enabled}
              onChange={(event) => {
                updateField(
                  "notification_enabled",
                  event.currentTarget.checked,
                );
              }}
            />
            <span class="label">截图后发送通知</span>
          </label>

          <label class="field">
            <span class="label">缓存保留上限</span>
            <input
              class="input"
              type="number"
              min="1"
              max="500"
              value={settings().cache_limit}
              onInput={(event) => {
                updateField(
                  "cache_limit",
                  Number(event.currentTarget.value || 1),
                );
              }}
            />
          </label>

          <label class="field checkboxField">
            <input
              class="checkbox"
              type="checkbox"
              checked={settings().launch_on_login}
              onChange={(event) => {
                updateField("launch_on_login", event.currentTarget.checked);
              }}
            />
            <span class="label">开机启动</span>
          </label>

          <div class="actions">
            <button
              class="button buttonPrimary"
              type="submit"
              disabled={saving()}
            >
              {saving() ? "保存中..." : "保存设置"}
            </button>
          </div>
        </form>

        <section class="selectionCard">
          <div class="selectionHeader">
            <h2 class="sectionTitle">最近一次选区</h2>
            <button
              class="miniButton"
              type="button"
              onClick={() => {
                void refreshLastSelection();
              }}
            >
              刷新
            </button>
          </div>
          {lastSelection() ? (
            <dl class="selectionGrid">
              <div class="selectionItem">
                <dt>X</dt>
                <dd>{lastSelection()!.x}</dd>
              </div>
              <div class="selectionItem">
                <dt>Y</dt>
                <dd>{lastSelection()!.y}</dd>
              </div>
              <div class="selectionItem">
                <dt>宽</dt>
                <dd>{lastSelection()!.width}</dd>
              </div>
              <div class="selectionItem">
                <dt>高</dt>
                <dd>{lastSelection()!.height}</dd>
              </div>
            </dl>
          ) : (
            <p class="selectionEmpty">还没有记录选区。先打开 overlay 拖一块区域。</p>
          )}
        </section>

        <section class="selectionCard">
          <div class="selectionHeader">
            <h2 class="sectionTitle">最近一次截图文件</h2>
            <button
              class="miniButton"
              type="button"
              onClick={() => {
                void refreshLastCaptureResult();
              }}
            >
              刷新
            </button>
          </div>
          {lastCaptureResult() ? (
            <dl class="selectionGrid">
              <div class="selectionItem">
                <dt>宽</dt>
                <dd>{lastCaptureResult()!.width}</dd>
              </div>
              <div class="selectionItem">
                <dt>高</dt>
                <dd>{lastCaptureResult()!.height}</dd>
              </div>
              <div class="selectionItem selectionWide">
                <dt>文件</dt>
                <dd class="pathValue">{lastCaptureResult()!.file_path}</dd>
              </div>
            </dl>
          ) : (
            <p class="selectionEmpty">还没有生成截图文件。先完成一次框选。</p>
          )}
        </section>

        <p class="status">{status()}</p>
      </section>
    </main>
  );
}
