import { appWindow } from '@tauri-apps/api/window';

function TitleBar() {
  const handleMinimize = async () => {
    await appWindow.minimize();
  };

  const handleClose = async () => {
    await appWindow.hide();
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
