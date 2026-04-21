import { useState, useEffect, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import TitleBar from './components/TitleBar';
import StatusBar from './components/StatusBar';
import Preview from './components/Preview';
import HistoryList from './components/HistoryList';

interface ClipboardContent {
  Text?: string;
  Image?: string;
}

interface HistoryItem {
  timestamp: string;
  content: ClipboardContent;
  id: string;
}

interface CommandResult<T> {
  success: boolean;
  data: T | null;
  error: string | null;
}

function App() {
  const [currentContent, setCurrentContent] = useState<ClipboardContent | null>(null);
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [isMonitoring, setIsMonitoring] = useState(true);
  const [copyCount, setCopyCount] = useState(0);

  const loadHistory = useCallback(async () => {
    try {
      const result: CommandResult<HistoryItem[]> = await invoke('get_history', { count: 20 });
      if (result.success && result.data) {
        setHistory(result.data);
      }
    } catch (error) {
      console.error('Failed to load history:', error);
    }
  }, []);

  const loadMonitoringStatus = useCallback(async () => {
    try {
      const result: CommandResult<boolean> = await invoke('get_monitoring_status');
      if (result.success && result.data !== null) {
        setIsMonitoring(result.data);
      }
    } catch (error) {
      console.error('Failed to get monitoring status:', error);
    }
  }, []);

  const loadCurrentContent = useCallback(async () => {
    try {
      const result: CommandResult<ClipboardContent | null> = await invoke('get_clipboard_content');
      if (result.success) {
        setCurrentContent(result.data);
      }
    } catch (error) {
      console.error('Failed to get clipboard content:', error);
    }
  }, []);

  useEffect(() => {
    loadHistory();
    loadMonitoringStatus();
    loadCurrentContent();

    const unlistenClipboard = listen<ClipboardContent>('clipboard-changed', (event) => {
      setCurrentContent(event.payload);
      loadHistory();
    });

    const unlistenStatus = listen<boolean>('monitoring-status-changed', (event) => {
      setIsMonitoring(event.payload);
    });

    return () => {
      unlistenClipboard.then((fn) => fn());
      unlistenStatus.then((fn) => fn());
    };
  }, [loadHistory, loadMonitoringStatus, loadCurrentContent]);

  const handleToggleMonitoring = async () => {
    try {
      const result: CommandResult<boolean> = await invoke('toggle_monitoring');
      if (result.success && result.data !== null) {
        setIsMonitoring(result.data);
      }
    } catch (error) {
      console.error('Failed to toggle monitoring:', error);
    }
  };

  const handleOpenDataDir = async () => {
    try {
      const result: CommandResult<void> = await invoke('open_data_directory');
      if (!result.success) {
        console.error('Failed to open data directory:', result.error);
      }
    } catch (error) {
      console.error('Failed to open data directory:', error);
    }
  };

  const handleCopyToClipboard = async () => {
    const newCount = copyCount + 1;
    const textToCopy = `测试文本 ${newCount} - ${new Date().toLocaleTimeString()}`;
    
    try {
      const result: CommandResult<void> = await invoke('copy_to_clipboard', { text: textToCopy });
      if (result.success) {
        setCopyCount(newCount);
        console.log(`已复制: ${textToCopy}`);
      }
    } catch (error) {
      console.error('Failed to copy to clipboard:', error);
    }
  };

  return (
    <div className="app-container">
      <TitleBar />
      <StatusBar isMonitoring={isMonitoring} onToggle={handleToggleMonitoring} />
      
      <div className="content-area">
        <div className="section">
          <div className="section-header">
            <span className="section-title">当前剪贴板内容</span>
            <button 
              className="btn btn-primary"
              onClick={handleCopyToClipboard}
            >
              复制测试文本 (点击: {copyCount})
            </button>
          </div>
          <Preview content={currentContent} />
        </div>

        <div className="section">
          <div className="section-header">
            <span className="section-title">历史记录</span>
            <button 
              className="btn btn-link"
              onClick={handleOpenDataDir}
            >
              打开数据目录
            </button>
          </div>
          <HistoryList items={history} />
        </div>
      </div>
    </div>
  );
}

export default App;
