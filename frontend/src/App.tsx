import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { open as openShell } from "@tauri-apps/plugin-shell";

type Status = "idle" | "processing" | "done" | "error";

interface ProcessConfig {
  input: string;
  output: string;
  confidence: number;
  min_duration: number;
  merge_gap: number;
  sample_rate: number;
}

const DEFAULT_CONFIG: Omit<ProcessConfig, "input" | "output"> = {
  confidence: 0.35,
  min_duration: 2.0,
  merge_gap: 3.0,
  sample_rate: 5,
};

const MODELS = [
  { id: "nano",   label: "YOLOv8 nano",   hint: "~6 MB · fastest",  file: "yolov8n.onnx" },
  { id: "small",  label: "YOLOv8 small",  hint: "~22 MB · balanced", file: "yolov8s.onnx" },
  { id: "medium", label: "YOLOv8 medium", hint: "~52 MB · accurate", file: "yolov8m.onnx" },
];

const sleep = (ms: number) => new Promise(resolve => window.setTimeout(resolve, ms));

export default function App() {
  const [videoPath, setVideoPath] = useState<string>("");
  const [outputPath, setOutputPath] = useState<string>("./output");
  const [modelType, setModelType] = useState<string>("nano");
  const [downloadedModels, setDownloadedModels] = useState<string[]>([]);
  const [downloadingModel, setDownloadingModel] = useState<string | null>(null);
  const [config, setConfig] = useState(DEFAULT_CONFIG);
  const [status, setStatus] = useState<Status>("idle");
  const [message, setMessage] = useState<string>("");
  const [activeTab, setActiveTab] = useState<"process" | "demo">("process");
  const [outputVideos, setOutputVideos] = useState<string[]>([]);

  const refreshModels = async () => {
    try {
      const models = await invoke<string[]>("get_available_models");
      setDownloadedModels(models);
      if (models.length > 0) {
        const firstModel = MODELS.find(m => models.includes(m.file));
        if (firstModel) setModelType(firstModel.id);
      }
    } catch {}
  };

  useEffect(() => { refreshModels(); }, []);

  const isDownloaded = (modelId: string) => {
    const model = MODELS.find(m => m.id === modelId);
    return model ? downloadedModels.includes(model.file) : false;
  };

  const waitForDownloadedModel = async (modelId: string, timeoutMs = 180_000) => {
    const model = MODELS.find(m => m.id === modelId);
    if (!model) return false;

    const startedAt = Date.now();
    while (Date.now() - startedAt < timeoutMs) {
      const models = await invoke<string[]>("get_available_models");
      setDownloadedModels(models);
      if (models.includes(model.file)) return true;
      await sleep(1000);
    }

    return false;
  };

  const pickVideo = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Video", extensions: ["mp4", "mkv", "avi", "mov"] }],
    });
    if (selected) setVideoPath(selected as string);
  };

  const pickOutput = async () => {
    const selected = await open({ directory: true });
    if (selected) setOutputPath(selected as string);
  };

  const handleSelectModel = (id: string) => {
    if (isDownloaded(id)) setModelType(id);
  };

  const handleDownloadModel = async (id: string) => {
    setDownloadingModel(id);
    setStatus("processing");
    setMessage("Downloading model, please wait…");
    try {
      await invoke("download_model_cmd", { modelType: id });
      await refreshModels();
      setModelType(id);
      setStatus("done");
      setMessage("Model downloaded");
    } catch (e) {
      const downloaded = await waitForDownloadedModel(id);
      if (downloaded) {
        setModelType(id);
        setStatus("done");
        setMessage("Model downloaded");
      } else {
        setStatus("error");
        setMessage(String(e));
      }
    } finally {
      setDownloadingModel(null);
    }
  };

  const loadOutputVideos = async () => {
    try {
      const videoName = videoPath.split("/").pop()?.replace(/\.[^.]+$/, "") ?? "";
      const videos = await invoke<string[]>("get_output_videos", { 
        outputDir: outputPath,
        videoName,
      });
      setOutputVideos(videos);
    } catch {}
  };

  const handleProcess = async () => {
    if (!videoPath) return;
    setStatus("processing");
    setMessage("");
    try {
      const result = await invoke<string>("process_video_cmd", {
        cfg: { input: videoPath, output: outputPath, ...config },
      });
      setStatus("done");
      setMessage(result);
      await loadOutputVideos();
    } catch (e) {
      setStatus("error");
      setMessage(String(e));
    }
  };

  const handleDemo = async () => {
    setStatus("processing");
    setMessage("");
    try {
      const result = await invoke<string>("run_demo_cmd", { output: outputPath });
      setStatus("done");
      setMessage(result);
      await loadOutputVideos();
    } catch (e) {
      setStatus("error");
      setMessage(String(e));
    }
  };

  const openVideo = async (path: string) => {
    try {
      await invoke("open_video_file", { path });
    } catch (e) {
      console.error("Failed to open video:", e);
    }
  };

  const fileName = (path: string) => path.split("/").pop() ?? path;

  return (
    <>
      <style>{`
        *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

        :root {
          --bg: #0a0a0a;
          --surface: #111111;
          --surface2: #1a1a1a;
          --border: #2a2a2a;
          --accent: #e8ff47;
          --accent2: #ff6b35;
          --text: #f0f0f0;
          --muted: #666;
          --success: #4ade80;
          --error: #f87171;
        }

        body {
          background: var(--bg); color: var(--text);
          font-family: 'Syne', 'Segoe UI', Ubuntu, sans-serif;
          min-height: 100vh; overflow-x: hidden;
        }

        .noise {
          position: fixed; inset: 0; pointer-events: none; z-index: 0; opacity: 0.03;
          background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)'/%3E%3C/svg%3E");
        }

        .app {
          position: relative; z-index: 1;
          max-width: 780px; margin: 0 auto;
          padding: 48px 24px 0;
          display: flex; flex-direction: column; min-height: 100vh;
        }

        .app-content { flex: 1; }

        .header { margin-bottom: 48px; }

        .logo { display: flex; align-items: center; gap: 12px; margin-bottom: 8px; }

        .logo-icon {
          width: 40px; height: 40px; background: var(--accent);
          clip-path: polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%);
          display: flex; align-items: center; justify-content: center; flex-shrink: 0;
        }

        .logo-icon svg { width: 20px; height: 20px; }

        h1 { font-size: 32px; font-weight: 800; letter-spacing: -1px; }
        h1 span { color: var(--accent); }

        .subtitle {
          font-family: 'Courier New', monospace; font-size: 12px;
          color: var(--muted); letter-spacing: 0.05em; margin-top: 4px;
        }

        .tabs {
          display: flex; gap: 2px; background: var(--surface);
          border: 1px solid var(--border); border-radius: 10px;
          padding: 4px; margin-bottom: 32px; width: fit-content;
        }

        .tab {
          padding: 8px 20px; border-radius: 7px; border: none;
          background: none; color: var(--muted); font-family: inherit;
          font-size: 14px; font-weight: 600; cursor: pointer; transition: all 0.15s;
        }

        .tab.active { background: var(--accent); color: #000; }

        .section {
          background: var(--surface); border: 1px solid var(--border);
          border-radius: 12px; padding: 24px; margin-bottom: 16px;
        }

        .section-label {
          font-family: 'Courier New', monospace; font-size: 11px; font-weight: 500;
          color: var(--muted); letter-spacing: 0.1em; text-transform: uppercase; margin-bottom: 16px;
        }

        .file-picker { display: flex; gap: 10px; align-items: center; }

        .file-path {
          flex: 1; background: var(--surface2); border: 1px solid var(--border);
          border-radius: 8px; padding: 10px 14px;
          font-family: 'Courier New', monospace; font-size: 12px; color: var(--text);
          white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
        }

        .file-path.empty { color: var(--muted); }

        .btn {
          padding: 10px 18px; border-radius: 8px; border: 1px solid var(--border);
          background: var(--surface2); color: var(--text); font-family: inherit;
          font-size: 13px; font-weight: 600; cursor: pointer; transition: all 0.15s; white-space: nowrap;
        }

        .btn:hover { border-color: var(--accent); color: var(--accent); }
        .btn:disabled { opacity: 0.4; cursor: not-allowed; }

        .btn-accent { background: var(--accent); border-color: var(--accent); color: #000; }
        .btn-accent:hover { background: #d4eb3a; border-color: #d4eb3a; color: #000; }
        .btn-accent:disabled { opacity: 0.4; cursor: not-allowed; }

        .btn-demo { background: var(--accent2); border-color: var(--accent2); color: #fff; }
        .btn-demo:hover { background: #e55a28; border-color: #e55a28; color: #fff; }

        .model-grid { display: flex; gap: 8px; }

        .model-card {
          flex: 1; border-radius: 10px; border: 1px solid var(--border);
          background: var(--surface2); padding: 14px; cursor: pointer;
          transition: all 0.15s;
        }

        .model-card.selected { border-color: var(--accent); background: rgba(232,255,71,0.06); }
        .model-card.not-downloaded { opacity: 0.7; }

        .model-card-name { font-size: 13px; font-weight: 700; color: var(--text); margin-bottom: 4px; }
        .model-card.selected .model-card-name { color: var(--accent); }

        .model-card-hint {
          font-family: 'Courier New', monospace; font-size: 10px;
          color: var(--muted); margin-bottom: 10px;
        }

        .badge {
          display: inline-flex; align-items: center;
          font-family: 'Courier New', monospace; font-size: 9px; font-weight: 600;
          padding: 2px 6px; border-radius: 4px; margin-bottom: 6px;
        }

        .badge-downloaded { background: rgba(74,222,128,0.15); color: var(--success); }
        .badge-missing { background: rgba(102,102,102,0.15); color: var(--muted); }

        .model-card-action {
          width: 100%; padding: 7px; border-radius: 6px; border: 1px solid var(--border);
          background: var(--surface); color: var(--muted); font-family: inherit;
          font-size: 11px; font-weight: 600; cursor: pointer; transition: all 0.15s; text-align: center;
        }

        .model-card-action.download { border-color: var(--accent2); color: var(--accent2); }
        .model-card-action.download:hover { background: var(--accent2); color: #fff; }
        .model-card-action.select { border-color: var(--accent); color: var(--accent); }
        .model-card-action.select:hover { background: var(--accent); color: #000; }
        .model-card-action.selected-active { background: var(--accent); border-color: var(--accent); color: #000; }
        .model-card-action:disabled { opacity: 0.5; cursor: not-allowed; }

        .sliders { display: flex; flex-direction: column; gap: 20px; }
        .slider-row { display: flex; align-items: center; gap: 16px; }
        .slider-label-wrap { width: 160px; flex-shrink: 0; }
        .slider-label { font-size: 13px; font-weight: 600; color: var(--text); }
        .slider-desc { font-family: 'Courier New', monospace; font-size: 10px; color: var(--muted); margin-top: 2px; }

        input[type="range"] {
          flex: 1; -webkit-appearance: none; height: 3px;
          background: var(--border); border-radius: 2px; outline: none;
        }

        input[type="range"]::-webkit-slider-thumb {
          -webkit-appearance: none; width: 16px; height: 16px; border-radius: 50%;
          background: var(--accent); cursor: pointer; transition: transform 0.15s;
        }

        input[type="range"]::-webkit-slider-thumb:hover { transform: scale(1.2); }

        .slider-value {
          font-family: 'Courier New', monospace; font-size: 13px; font-weight: 500;
          color: var(--accent); width: 48px; text-align: right; flex-shrink: 0;
        }

        .action-row { display: flex; gap: 12px; align-items: center; margin-top: 8px; }

        .status-bar {
          margin-top: 16px; padding: 14px 18px; border-radius: 10px;
          border: 1px solid var(--border); font-family: 'Courier New', monospace;
          font-size: 12px; display: flex; align-items: center; gap: 10px;
          animation: fadeIn 0.2s ease;
        }

        .status-bar.processing { border-color: var(--accent); background: rgba(232,255,71,0.04); color: var(--accent); }
        .status-bar.done { border-color: var(--success); background: rgba(74,222,128,0.04); color: var(--success); }
        .status-bar.error { border-color: var(--error); background: rgba(248,113,113,0.04); color: var(--error); }

        .spinner {
          width: 14px; height: 14px; border: 2px solid rgba(232,255,71,0.2);
          border-top-color: var(--accent); border-radius: 50%;
          animation: spin 0.7s linear infinite; flex-shrink: 0;
        }

        .videos-section {
          margin-top: 16px; animation: fadeIn 0.3s ease;
        }

        .videos-header {
          display: flex; align-items: center; justify-content: space-between;
          margin-bottom: 12px;
        }

        .videos-title {
          font-family: 'Courier New', monospace; font-size: 11px; font-weight: 600;
          color: var(--success); letter-spacing: 0.1em; text-transform: uppercase;
        }

        .videos-grid { display: flex; flex-direction: column; gap: 8px; }

        .video-item {
          display: flex; align-items: center; gap: 12px;
          background: var(--surface); border: 1px solid var(--border);
          border-radius: 10px; padding: 12px 16px; cursor: pointer;
          transition: all 0.15s;
        }

        .video-item:hover { border-color: var(--success); background: rgba(74,222,128,0.04); }

        .video-icon {
          width: 36px; height: 36px; border-radius: 8px;
          background: rgba(74,222,128,0.1); border: 1px solid rgba(74,222,128,0.2);
          display: flex; align-items: center; justify-content: center;
          flex-shrink: 0; color: var(--success); font-size: 16px;
        }

        .video-name {
          flex: 1; font-family: 'Courier New', monospace; font-size: 12px;
          color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
        }

        .video-open {
          font-family: 'Courier New', monospace; font-size: 11px;
          color: var(--muted); flex-shrink: 0;
        }

        .video-item:hover .video-open { color: var(--success); }

        .demo-desc { font-size: 13px; color: var(--muted); line-height: 1.6; margin-bottom: 20px; }
        .demo-desc strong { color: var(--text); }
        .divider { height: 1px; background: var(--border); margin: 20px 0; }

        .footer {
          margin-top: 64px; padding: 40px 0 48px;
          border-top: 1px solid var(--border);
        }

        .footer-inner {
          display: flex; align-items: flex-start; gap: 32px;
        }

        .footer-content { flex: 1; text-align: start;}

        .footer-brand {
          font-size: 14px; font-weight: 700; color: var(--text);
          margin-bottom: 10px; display: flex; align-items: center;
        }

        .footer-brand span { color: var(--accent); }

        .footer-disclaimer {
          font-family: 'Courier New', monospace; font-size: 11px;
          color: var(--muted); line-height: 1.7; margin-bottom: 14px;
        }

        .footer-disclaimer strong { color: var(--text); }

        .footer-github {
          display: inline-flex; align-items: start; gap: 6px;
          font-family: 'Courier New', monospace; font-size: 11px;
          color: var(--muted); text-decoration: none;
          border: 1px solid var(--border); border-radius: 6px;
          padding: 5px 10px; transition: all 0.15s;
        }

        .footer-github:hover { border-color: var(--accent); color: var(--accent); }

        @keyframes spin { to { transform: rotate(360deg); } }
        @keyframes fadeIn { from { opacity: 0; transform: translateY(4px); } to { opacity: 1; transform: none; } }
      `}</style>

      <div className="noise" />

      <div className="app">
        <div className="app-content">
          <div className="header">
            <div className="logo">
              <div className="logo-icon">
                <svg viewBox="0 0 24 24" fill="none" stroke="#000" strokeWidth="2.5">
                  <path d="M15 10l5-5-5-5v3H4v4h11v3z"/>
                  <path d="M9 14l-5 5 5 5v-3h11v-4H9v-3z"/>
                </svg>
              </div>
              <h1>Human<span>Cut</span></h1>
            </div>
            <div className="subtitle">// yolov8 · smart video trimming · onnx runtime</div>
          </div>

          <div className="tabs">
            <button className={`tab ${activeTab === "process" ? "active" : ""}`} onClick={() => setActiveTab("process")}>
              Process Video
            </button>
            <button className={`tab ${activeTab === "demo" ? "active" : ""}`} onClick={() => setActiveTab("demo")}>
              Demo
            </button>
          </div>

          {activeTab === "process" && (
            <>
              <div className="section">
                <div className="section-label">Input Video</div>
                <div className="file-picker">
                  <div className={`file-path ${!videoPath ? "empty" : ""}`}>
                    {videoPath || "No video selected…"}
                  </div>
                  <button className="btn" onClick={pickVideo}>Browse</button>
                </div>
              </div>

              <div className="section">
                <div className="section-label">Output Folder</div>
                <div className="file-picker">
                  <div className="file-path">{outputPath}</div>
                  <button className="btn" onClick={pickOutput}>Browse</button>
                </div>
              </div>

              <div className="section">
                <div className="section-label">YOLO Model</div>
                <div className="model-grid">
                  {MODELS.map((m) => {
                    const downloaded = isDownloaded(m.id);
                    const isSelected = modelType === m.id;
                    const isDownloading = downloadingModel === m.id;
                    return (
                      <div
                        key={m.id}
                        className={`model-card ${isSelected ? "selected" : ""} ${!downloaded ? "not-downloaded" : ""}`}
                        onClick={() => downloaded && handleSelectModel(m.id)}
                      >
                        <div className="model-card-name">{m.label}</div>
                        <div className="model-card-hint">{m.hint}</div>
                        <div className={`badge ${downloaded ? "badge-downloaded" : "badge-missing"}`}>
                          {downloaded ? "✓ ready" : "not downloaded"}
                        </div>
                        <button
                          className={`model-card-action ${!downloaded ? "download" : isSelected ? "selected-active" : "select"}`}
                          disabled={downloadingModel !== null}
                          onClick={(e) => {
                            e.stopPropagation();
                            if (!downloaded) handleDownloadModel(m.id);
                            else handleSelectModel(m.id);
                          }}
                        >
                          {isDownloading ? "Downloading…" : !downloaded ? "↓ Download" : isSelected ? "✓ Selected" : "Select"}
                        </button>
                      </div>
                    );
                  })}
                </div>
              </div>

              <div className="section">
                <div className="section-label">Detection Parameters</div>
                <div className="sliders">
                  <div className="slider-row">
                    <div className="slider-label-wrap">
                      <div className="slider-label">Confidence</div>
                      <div className="slider-desc">detection threshold</div>
                    </div>
                    <input type="range" min={0.1} max={0.9} step={0.05} value={config.confidence}
                      onChange={e => setConfig(c => ({ ...c, confidence: +e.target.value }))} />
                    <div className="slider-value">{config.confidence.toFixed(2)}</div>
                  </div>
                  <div className="slider-row">
                    <div className="slider-label-wrap">
                      <div className="slider-label">Min Duration</div>
                      <div className="slider-desc">seconds per clip</div>
                    </div>
                    <input type="range" min={0.5} max={30} step={0.5} value={config.min_duration}
                      onChange={e => setConfig(c => ({ ...c, min_duration: +e.target.value }))} />
                    <div className="slider-value">{config.min_duration}s</div>
                  </div>
                  <div className="slider-row">
                    <div className="slider-label-wrap">
                      <div className="slider-label">Merge Gap</div>
                      <div className="slider-desc">join nearby clips</div>
                    </div>
                    <input type="range" min={0.5} max={30} step={0.5} value={config.merge_gap}
                      onChange={e => setConfig(c => ({ ...c, merge_gap: +e.target.value }))} />
                    <div className="slider-value">{config.merge_gap}s</div>
                  </div>
                  <div className="slider-row">
                    <div className="slider-label-wrap">
                      <div className="slider-label">Sample Rate</div>
                      <div className="slider-desc">process every Nth frame</div>
                    </div>
                    <input type="range" min={1} max={30} step={1} value={config.sample_rate}
                      onChange={e => setConfig(c => ({ ...c, sample_rate: +e.target.value }))} />
                    <div className="slider-value">1/{config.sample_rate}</div>
                  </div>
                </div>
              </div>

              <div className="action-row">
                <button
                  className="btn btn-accent"
                  onClick={handleProcess}
                  disabled={!videoPath || status === "processing" || !isDownloaded(modelType)}
                  style={{ flex: 1, padding: "14px" }}
                >
                  {status === "processing" ? "Processing…" : "▶  Process Video"}
                </button>
              </div>
            </>
          )}

          {activeTab === "demo" && (
            <div className="section">
              <div className="section-label">Demo Mode</div>
              <div className="demo-desc">
                Automatically downloads a <strong>sample video with people</strong> and runs the full detection pipeline.
              </div>
              <div className="section-label">Output Folder</div>
              <div className="file-picker" style={{ marginBottom: 20 }}>
                <div className="file-path">{outputPath}</div>
                <button className="btn" onClick={pickOutput}>Browse</button>
              </div>
              <button
                className="btn btn-demo"
                onClick={handleDemo}
                disabled={status === "processing"}
                style={{ width: "100%", padding: "14px", fontSize: 15 }}
              >
                {status === "processing" ? "Running demo…" : "▶  Run Demo"}
              </button>
            </div>
          )}

          {status !== "idle" && (
            <div className={`status-bar ${status}`}>
              {status === "processing" && <div className="spinner" />}
              {status === "done" && "✓"}
              {status === "error" && "✗"}
              <span>{status === "processing" ? message || "Processing, please wait…" : message}</span>
            </div>
          )}

          {status === "done" && outputVideos.length > 0 && (
          <div className="videos-section">
            <div className="videos-header">
              <div className="videos-title">✓ {outputVideos.length} segment{outputVideos.length > 1 ? "s" : ""} ready</div>
            </div>
            <div className="videos-grid">
              {outputVideos.map((path) => (
                <div key={path} className="video-item" onClick={() => openVideo(path)}>
                  <div className="video-icon">▶</div>
                  <div className="video-name">{fileName(path)}</div>
                  <div className="video-open">open ↗</div>
                </div>
              ))}
            </div>
          </div>
        )}
        </div>

        <footer className="footer">
          <div className="footer-inner">
            <div className="logo-icon">
                <svg viewBox="0 0 24 24" fill="none" stroke="#000" strokeWidth="2.5">
                  <path d="M15 10l5-5-5-5v3H4v4h11v3z"/>
                  <path d="M9 14l-5 5 5 5v-3h11v-4H9v-3z"/>
                </svg>
              </div>
            <div className="footer-content">
              <div className="footer-brand">
                Human<span>Cut</span>
              </div>
              <div className="footer-disclaimer">
                <strong>Your privacy is protected.</strong> All video processing happens entirely on your device using a local YOLO model.
                No video files, frames, or personal data are ever uploaded to any server or sent over the internet.
                Everything stays on your machine.
              </div>
              <a
                className="footer-github"
                href="https://github.com/GeekP1e"
                onClick={(e) => { e.preventDefault(); openShell("https://github.com/GeekP1e"); }}
              >
                <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M12 0C5.37 0 0 5.37 0 12c0 5.3 3.44 9.8 8.21 11.39.6.11.82-.26.82-.58v-2.03c-3.34.72-4.04-1.61-4.04-1.61-.55-1.39-1.34-1.76-1.34-1.76-1.09-.75.08-.73.08-.73 1.2.08 1.84 1.24 1.84 1.24 1.07 1.83 2.81 1.3 3.49 1 .11-.78.42-1.3.76-1.6-2.67-.3-5.47-1.33-5.47-5.93 0-1.31.47-2.38 1.24-3.22-.13-.3-.54-1.52.12-3.18 0 0 1.01-.32 3.3 1.23a11.5 11.5 0 013-.4c1.02.004 2.04.14 3 .4 2.28-1.55 3.29-1.23 3.29-1.23.66 1.66.25 2.88.12 3.18.77.84 1.24 1.91 1.24 3.22 0 4.61-2.81 5.63-5.48 5.92.43.37.81 1.1.81 2.22v3.29c0 .32.22.7.83.58C20.57 21.8 24 17.3 24 12c0-6.63-5.37-12-12-12z"/>
                </svg>
                github.com/GeekP1e
              </a>
            </div>
          </div>
        </footer>
      </div>
    </>
  );
}
