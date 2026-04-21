interface ClipboardContent {
  Text?: string;
  Image?: string;
}

interface PreviewProps {
  content: ClipboardContent | null;
}

function Preview({ content }: PreviewProps) {
  if (!content) {
    return (
      <div className="preview-container">
        <div className="preview-empty">
          暂无内容（预览已清空或剪贴板为空）
        </div>
      </div>
    );
  }

  if (content.Text) {
    return (
      <div className="preview-container">
        <pre className="preview-text">{content.Text}</pre>
      </div>
    );
  }

  if (content.Image) {
    return (
      <div className="preview-container">
        <div className="preview-empty">
          <span style={{ color: '#4ade80' }}>[图片] {content.Image}</span>
        </div>
      </div>
    );
  }

  return (
    <div className="preview-container">
      <div className="preview-empty">未知内容类型</div>
    </div>
  );
}

export default Preview;
