interface ClipboardContent {
  Text?: string;
  Image?: string;
}

interface HistoryItem {
  timestamp: string;
  content: ClipboardContent;
  id: string;
}

interface HistoryListProps {
  items: HistoryItem[];
}

function HistoryList({ items }: HistoryListProps) {
  if (items.length === 0) {
    return (
      <div className="history-list">
        <div className="history-empty">暂无历史记录</div>
      </div>
    );
  }

  const getContentDisplay = (content: ClipboardContent): { text: string; isImage: boolean } => {
    if (content.Text) {
      return { 
        text: content.Text.length > 80 ? content.Text.substring(0, 80) + '...' : content.Text,
        isImage: false 
      };
    }
    if (content.Image) {
      return { text: `[图片] ${content.Image}`, isImage: true };
    }
    return { text: '未知内容', isImage: false };
  };

  return (
    <div className="history-list">
      {items.map((item) => {
        const { text, isImage } = getContentDisplay(item.content);
        return (
          <div key={item.id} className="history-item">
            <div className="history-timestamp">{item.timestamp}</div>
            <div className={`history-content ${isImage ? 'image' : ''}`}>
              {text}
            </div>
          </div>
        );
      })}
    </div>
  );
}

export default HistoryList;
