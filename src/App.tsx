import { useState, useEffect, useCallback, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
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

interface LogEntry {
  id: string;
  timestamp: string;
  message: string;
  type: 'info' | 'warning' | 'error' | 'debug';
}

function App() {
  const [currentContent, setCurrentContent] = useState<ClipboardContent | null>(null);
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [isMonitoring, setIsMonitoring] = useState(true);
  const [copyCount, setCopyCount] = useState(0);
  
  const [currentTime, setCurrentTime] = useState<string>('');
  const [randomTexts, setRandomTexts] = useState<string[]>([]);
  const [uiLogs, setUiLogs] = useState<LogEntry[]>([]);
  const [pollingContent, setPollingContent] = useState<ClipboardContent | null>(null);
  const [pollingCount, setPollingCount] = useState(0);
  
  const unlistenRefs = useRef<{
    clipboardGlobal: (() => void) | null;
    clipboardWindow: (() => void) | null;
    statusGlobal: (() => void) | null;
    statusWindow: (() => void) | null;
    closeRequested: (() => void) | null;
  }>({
    clipboardGlobal: null,
    clipboardWindow: null,
    statusGlobal: null,
    statusWindow: null,
    closeRequested: null,
  });
  const mountedRef = useRef(true);

  const addLog = useCallback((message: string, type: LogEntry['type'] = 'info') => {
    const entry: LogEntry = {
      id: Date.now().toString() + Math.random().toString(36),
      timestamp: new Date().toLocaleTimeString(),
      message,
      type,
    };
    setUiLogs(prev => [entry, ...prev].slice(0, 100));
    console.log(`[UI Log] [${type.toUpperCase()}] ${message}`);
  }, []);

  const randomString = useCallback((): string => {
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let result = '';
    for (let i = 0; i < 8; i++) {
      result += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return result;
  }, []);

  const loadHistory = useCallback(async () => {
    try {
      const result: CommandResult<HistoryItem[]> = await invoke('get_history', { count: 20 });
      if (result.success && result.data) {
        setHistory(result.data);
        addLog(`历史记录加载成功，共 ${result.data.length} 条`, 'info');
      }
    } catch (error) {
      addLog(`加载历史记录失败: ${error}`, 'error');
    }
  }, [addLog]);

  const loadMonitoringStatus = useCallback(async () => {
    try {
      const result: CommandResult<boolean> = await invoke('get_monitoring_status');
      if (result.success && result.data !== null) {
        setIsMonitoring(result.data);
        addLog(`监听状态: ${result.data ? '运行中' : '已停止'}`, 'info');
      }
    } catch (error) {
      addLog(`获取监听状态失败: ${error}`, 'error');
    }
  }, [addLog]);

  const loadCurrentContent = useCallback(async () => {
    try {
      addLog('调用 get_clipboard_content...', 'debug');
      const result: CommandResult<ClipboardContent | null> = await invoke('get_clipboard_content');
      addLog(`get_clipboard_content 返回: ${result.success}, 数据: ${JSON.stringify(result.data).substring(0, 50)}`, 'debug');
      if (result.success) {
        setCurrentContent(result.data);
        if (result.data) {
          addLog(`当前剪贴板内容更新: ${JSON.stringify(result.data).substring(0, 80)}`, 'info');
        }
      }
    } catch (error) {
      addLog(`获取剪贴板内容失败: ${error}`, 'error');
    }
  }, [addLog]);

  const handleClipboardChanged = useCallback((source: string, payload: ClipboardContent) => {
    addLog(`========== [${source}] 收到 clipboard-changed 事件 ==========`, 'warning');
    addLog(`事件 payload: ${JSON.stringify(payload).substring(0, 100)}`, 'info');
    
    if (mountedRef.current) {
      setCurrentContent(payload);
      loadHistory();
    }
  }, [addLog, loadHistory]);

  const handleStatusChanged = useCallback((source: string, payload: boolean) => {
    addLog(`[${source}] 收到 monitoring-status-changed 事件: ${payload}`, 'info');
    if (mountedRef.current) {
      setIsMonitoring(payload);
    }
  }, [addLog]);

  useEffect(() => {
    addLog('=== App 组件挂载 ===', 'info');
    mountedRef.current = true;

    loadHistory();
    loadMonitoringStatus();
    loadCurrentContent();

    const setupListeners = async () => {
      addLog('设置事件监听器...', 'debug');
      
      try {
        const webviewWindow = getCurrentWebviewWindow();
        addLog(`WebviewWindow label: ${webviewWindow.label}`, 'debug');

        addLog('设置 close-requested 事件监听...', 'debug');
        const unlistenClose = await webviewWindow.listen('tauri://close-requested', (_event) => {
          addLog('========== 收到 close-requested 事件 - 隐藏窗口而不是关闭 ==========', 'warning');
          webviewWindow.hide().catch(err => {
            addLog(`隐藏窗口失败: ${err}`, 'error');
          });
        });
        unlistenRefs.current.closeRequested = unlistenClose;
        addLog('close-requested 监听器已设置', 'info');

        addLog('设置全局事件监听 (listen from @tauri-apps/api/event)...', 'debug');
        const unlistenClipboardGlobal = await listen<ClipboardContent>('clipboard-changed', (event) => {
          handleClipboardChanged('全局 listen', event.payload);
        });
        unlistenRefs.current.clipboardGlobal = unlistenClipboardGlobal;
        addLog('全局 clipboard-changed 监听器已设置', 'info');

        const unlistenStatusGlobal = await listen<boolean>('monitoring-status-changed', (event) => {
          handleStatusChanged('全局 listen', event.payload);
        });
        unlistenRefs.current.statusGlobal = unlistenStatusGlobal;
        addLog('全局 monitoring-status-changed 监听器已设置', 'info');

        addLog('设置窗口事件监听 (webviewWindow.listen)...', 'debug');
        const unlistenClipboardWindow = await webviewWindow.listen<ClipboardContent>('clipboard-changed', (event) => {
          handleClipboardChanged('窗口 listen', event.payload as ClipboardContent);
        });
        unlistenRefs.current.clipboardWindow = unlistenClipboardWindow;
        addLog('窗口 clipboard-changed 监听器已设置', 'info');

        const unlistenStatusWindow = await webviewWindow.listen<boolean>('monitoring-status-changed', (event) => {
          handleStatusChanged('窗口 listen', event.payload as boolean);
        });
        unlistenRefs.current.statusWindow = unlistenStatusWindow;
        addLog('窗口 monitoring-status-changed 监听器已设置', 'info');

        addLog('✅ 所有事件监听器设置完成', 'info');
        
      } catch (error) {
        addLog(`设置监听器失败: ${error}`, 'error');
      }
    };

    setupListeners();

    return () => {
      addLog('=== App 组件卸载 ===', 'warning');
      mountedRef.current = false;
      
      if (unlistenRefs.current.closeRequested) {
        addLog('取消监听 close-requested', 'debug');
        unlistenRefs.current.closeRequested();
        unlistenRefs.current.closeRequested = null;
      }
      if (unlistenRefs.current.clipboardGlobal) {
        addLog('取消监听全局 clipboard-changed', 'debug');
        unlistenRefs.current.clipboardGlobal();
        unlistenRefs.current.clipboardGlobal = null;
      }
      if (unlistenRefs.current.clipboardWindow) {
        addLog('取消监听窗口 clipboard-changed', 'debug');
        unlistenRefs.current.clipboardWindow();
        unlistenRefs.current.clipboardWindow = null;
      }
      if (unlistenRefs.current.statusGlobal) {
        addLog('取消监听全局 monitoring-status-changed', 'debug');
        unlistenRefs.current.statusGlobal();
        unlistenRefs.current.statusGlobal = null;
      }
      if (unlistenRefs.current.statusWindow) {
        addLog('取消监听窗口 monitoring-status-changed', 'debug');
        unlistenRefs.current.statusWindow();
        unlistenRefs.current.statusWindow = null;
      }
    };
  }, [addLog, handleClipboardChanged, handleStatusChanged, loadHistory, loadMonitoringStatus, loadCurrentContent]);

  useEffect(() => {
    addLog('启动实时时间更新定时器', 'debug');
    const timeInterval = setInterval(() => {
      const now = new Date().toLocaleTimeString('zh-CN', {
        hour12: false,
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit'
      });
      setCurrentTime(now);
    }, 1000);

    return () => {
      clearInterval(timeInterval);
      addLog('停止实时时间更新定时器', 'debug');
    };
  }, [addLog]);

  useEffect(() => {
    addLog('启动随机文本定时器（每5秒）', 'debug');
    const textInterval = setInterval(() => {
      const newText = randomString();
      const time = new Date().toLocaleTimeString();
      const entry = `[${time}] ${newText}`;
      setRandomTexts(prev => [entry, ...prev].slice(0, 20));
      addLog(`添加随机文本: ${newText}`, 'info');
    }, 5000);

    return () => {
      clearInterval(textInterval);
      addLog('停止随机文本定时器', 'debug');
    };
  }, [addLog, randomString]);

  useEffect(() => {
    addLog('启动轮询剪贴板定时器（每2秒）', 'debug');
    const pollingInterval = setInterval(async () => {
      setPollingCount(prev => prev + 1);
      
      try {
        const result: CommandResult<ClipboardContent | null> = await invoke('get_clipboard_content');
        if (result.success && result.data) {
          const oldContent = pollingContent;
          const newContent = result.data;
          
          setPollingContent(newContent);
          
          const oldStr = JSON.stringify(oldContent);
          const newStr = JSON.stringify(newContent);
          
          if (oldStr !== newStr) {
            addLog(`轮询检测到剪贴板变化!`, 'warning');
            addLog(`旧内容: ${oldStr.substring(0, 80)}`, 'debug');
            addLog(`新内容: ${newStr.substring(0, 80)}`, 'info');
            
            setCurrentContent(newContent);
          }
        }
      } catch (error) {
        addLog(`轮询剪贴板失败: ${error}`, 'error');
      }
    }, 2000);

    return () => {
      clearInterval(pollingInterval);
      addLog('停止轮询剪贴板定时器', 'debug');
    };
  }, [addLog, pollingContent]);

  const handleToggleMonitoring = async () => {
    addLog('点击切换监听状态', 'debug');
    try {
      const result: CommandResult<boolean> = await invoke('toggle_monitoring');
      if (result.success && result.data !== null) {
        setIsMonitoring(result.data);
        addLog(`监听状态已切换: ${result.data ? '运行中' : '已停止'}`, 'info');
      }
    } catch (error) {
      addLog(`切换监听状态失败: ${error}`, 'error');
    }
  };

  const handleOpenDataDir = async () => {
    addLog('点击打开数据目录', 'debug');
    try {
      const result: CommandResult<void> = await invoke('open_data_directory');
      if (!result.success) {
        addLog(`打开数据目录失败: ${result.error}`, 'error');
      } else {
        addLog('数据目录已打开', 'info');
      }
    } catch (error) {
      addLog(`打开数据目录失败: ${error}`, 'error');
    }
  };

  const handleCopyToClipboard = async () => {
    const newCount = copyCount + 1;
    const textToCopy = `测试文本 ${newCount} - ${new Date().toLocaleTimeString()}`;
    
    addLog(`点击复制测试文本: ${textToCopy}`, 'debug');
    
    try {
      const result: CommandResult<void> = await invoke('copy_to_clipboard', { text: textToCopy });
      if (result.success) {
        setCopyCount(newCount);
        addLog(`已复制到剪贴板: ${textToCopy}`, 'info');
      } else {
        addLog(`复制失败: ${result.error}`, 'error');
      }
    } catch (error) {
      addLog(`复制失败: ${error}`, 'error');
    }
  };

  const handleRefresh = async () => {
    addLog('手动刷新所有数据', 'info');
    await loadCurrentContent();
    await loadHistory();
    await loadMonitoringStatus();
  };

  const getLogColor = (type: LogEntry['type']) => {
    switch (type) {
      case 'error': return '#ef4444';
      case 'warning': return '#f59e0b';
      case 'debug': return '#6b7280';
      default: return '#4ade80';
    }
  };

  return (
    <div className="app-container">
      <TitleBar />
      <StatusBar isMonitoring={isMonitoring} onToggle={handleToggleMonitoring} />
      
      <div className="content-area">
        <div className="section debug-section">
          <div className="section-header">
            <span className="section-title">🔧 调试面板</span>
            <div className="debug-controls">
              <button className="btn btn-small" onClick={handleRefresh}>
                🔄 刷新
              </button>
              <span className="polling-count">轮询: {pollingCount} 次</span>
            </div>
          </div>
          
          <div className="debug-grid">
            <div className="debug-panel">
              <div className="debug-panel-header">
                <span>⏰ 实时时间</span>
              </div>
              <div className="clock-display">
                {currentTime || '--:--:--'}
              </div>
            </div>
            
            <div className="debug-panel">
              <div className="debug-panel-header">
                <span>📋 轮询剪贴板</span>
              </div>
              <div className="polling-display">
                {pollingContent ? (
                  <div className="polling-content">
                    {pollingContent.Text ? (
                      <span className="polling-text">{pollingContent.Text.substring(0, 60)}</span>
                    ) : pollingContent.Image ? (
                      <span className="polling-image">🖼️ 图片</span>
                    ) : null}
                  </div>
                ) : (
                  <span className="polling-empty">无内容</span>
                )}
              </div>
            </div>
          </div>

          <div className="debug-grid">
            <div className="debug-panel">
              <div className="debug-panel-header">
                <span>🎲 随机文本（每5秒）</span>
              </div>
              <div className="random-texts">
                {randomTexts.length === 0 ? (
                  <span className="random-empty">等待生成...</span>
                ) : (
                  randomTexts.map((text, index) => (
                    <div key={index} className="random-text-item">{text}</div>
                  ))
                )}
              </div>
            </div>
            
            <div className="debug-panel">
              <div className="debug-panel-header">
                <span>📝 UI 日志</span>
              </div>
              <div className="ui-logs">
                {uiLogs.length === 0 ? (
                  <span className="logs-empty">暂无日志</span>
                ) : (
                  uiLogs.map((log) => (
                    <div key={log.id} className="log-item" style={{ color: getLogColor(log.type) }}>
                      <span className="log-time">[{log.timestamp}]</span>
                      <span className="log-message">{log.message}</span>
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>
        </div>

        <div className="section">
          <div className="section-header">
            <span className="section-title">当前剪贴板内容（事件驱动）</span>
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
