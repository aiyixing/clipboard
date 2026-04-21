interface StatusBarProps {
  isMonitoring: boolean;
  onToggle: () => void;
}

function StatusBar({ isMonitoring, onToggle }: StatusBarProps) {
  return (
    <div className="status-bar">
      <div className="status-indicator">
        <div className={`status-dot ${isMonitoring ? 'running' : 'stopped'}`} />
        <span className="status-text">
          {isMonitoring ? '运行中' : '已暂停'}
        </span>
      </div>
      <button 
        className="btn btn-primary"
        onClick={onToggle}
      >
        {isMonitoring ? '暂停监听' : '恢复监听'}
      </button>
    </div>
  );
}

export default StatusBar;
