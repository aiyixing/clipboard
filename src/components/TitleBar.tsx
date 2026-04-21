import { getCurrentWindow } from '@tauri-apps/api/window';

function TitleBar() {
  const handleMinimize = async () => {
    const window = getCurrentWindow();
    await window.minimize();
  };

  const handleClose = async () => {
    const window = getCurrentWindow();
    await window.hide();
  };

  return (
    <div className="title-bar">
      <span className="title">Clipboard Monitor</span>
      <div className="window-controls">
        <button className="window-btn" onClick={handleMinimize}>
          _
        </button>
        <button className="window-btn close" onClick={handleClose}>
          ✕
        </button>
      </div>
    </div>
  );
}

export default TitleBar;
