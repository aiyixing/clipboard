import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

function TitleBar() {
  const handleMinimize = async () => {
    const webviewWindow = getCurrentWebviewWindow();
    await webviewWindow.minimize();
  };

  const handleClose = async () => {
    const webviewWindow = getCurrentWebviewWindow();
    console.log('Closing window - hiding to tray');
    await webviewWindow.hide();
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
