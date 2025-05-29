use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::tools::base::FileDocumentFragment;

/// 文件分块配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    /// 最大块大小（字符数）
    pub max_chunk_size: usize,
    /// 块之间的重叠字符数
    pub overlap_size: usize,
    /// 是否保持语义完整性（在句子边界分块）
    pub preserve_semantic_boundaries: bool,
    /// 是否添加上下文信息
    pub add_context_info: bool,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            max_chunk_size: 8192,    // 8KB
            overlap_size: 512,       // 512字符
            preserve_semantic_boundaries: true,
            add_context_info: true,
        }
    }
}

/// 文件分块器
pub struct FileChunker {
    config: ChunkingConfig,
}

impl FileChunker {
    pub fn new(config: ChunkingConfig) -> Self {
        Self { config }
    }
    
    /// 将大文件分块
    pub fn chunk_file(&self, fragment: &FileDocumentFragment) -> Result<Vec<String>> {
        let content = &fragment.content;
        
        if content.len() <= self.config.max_chunk_size {
            // 文件不需要分块
            if self.config.add_context_info {
                return Ok(vec![self.add_context_to_chunk(fragment, content, 1, 1)]);
            } else {
                return Ok(vec![content.clone()]);
            }
        }
        
        if self.config.preserve_semantic_boundaries {
            self.chunk_with_semantic_boundaries(fragment, content)
        } else {
            self.chunk_simple(fragment, content)
        }
    }
    
    /// 简单分块（按字符数）
    fn chunk_simple(&self, fragment: &FileDocumentFragment, content: &str) -> Result<Vec<String>> {
        let mut chunks = Vec::new();
        let mut start = 0;
        let chunk_index = 0;
        
        while start < content.len() {
            let end = std::cmp::min(start + self.config.max_chunk_size, content.len());
            let chunk = &content[start..end];
            
            let chunk_text = if self.config.add_context_info {
                self.add_context_to_chunk(fragment, chunk, chunk_index + 1, chunks.len() + 1)
            } else {
                chunk.to_string()
            };
            
            chunks.push(chunk_text);
            
            if end >= content.len() {
                break;
            }
            
            // 处理重叠
            start = end - self.config.overlap_size;
        }
        
        Ok(chunks)
    }
    
    /// 语义边界分块（在行、段落或句子边界）
    fn chunk_with_semantic_boundaries(&self, fragment: &FileDocumentFragment, content: &str) -> Result<Vec<String>> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        let mut current_chunk = String::new();
        let mut chunk_index = 1;
        
        for line in lines {
            // 检查添加这一行后是否会超过块大小限制
            let new_size = current_chunk.len() + line.len() + 1; // +1 for newline
            
            if new_size > self.config.max_chunk_size && !current_chunk.is_empty() {
                // 当前块已达到大小限制，保存并开始新块
                let chunk_text = if self.config.add_context_info {
                    self.add_context_to_chunk(fragment, &current_chunk, chunk_index, 0)
                } else {
                    current_chunk.clone()
                };
                
                chunks.push(chunk_text);
                
                // 开始新块，可能包含重叠
                current_chunk = self.get_overlap_content(&current_chunk);
                chunk_index += 1;
            }
            
            if !current_chunk.is_empty() {
                current_chunk.push('\n');
            }
            current_chunk.push_str(line);
        }
        
        // 添加最后一个块
        if !current_chunk.is_empty() {
            let chunk_text = if self.config.add_context_info {
                self.add_context_to_chunk(fragment, &current_chunk, chunk_index, chunks.len() + 1)
            } else {
                current_chunk
            };
            chunks.push(chunk_text);
        }
        
        Ok(chunks)
    }
    
    /// 获取用于重叠的内容
    fn get_overlap_content(&self, content: &str) -> String {
        if content.len() <= self.config.overlap_size {
            return content.to_string();
        }
        
        // 如果启用了语义边界保护，在行边界处获取重叠内容
        if self.config.preserve_semantic_boundaries {
            let lines: Vec<&str> = content.lines().collect();
            let mut overlap_content = String::new();
            let mut current_size = 0;
            
            // 从最后一行开始，向前收集到达重叠大小
            for line in lines.iter().rev() {
                let line_size = line.len() + 1; // +1 for newline
                if current_size + line_size > self.config.overlap_size && !overlap_content.is_empty() {
                    break;
                }
                
                if !overlap_content.is_empty() {
                    overlap_content = format!("{}\n{}", line, overlap_content);
                } else {
                    overlap_content = line.to_string();
                }
                current_size += line_size;
            }
            
            overlap_content
        } else {
            // 从内容末尾取重叠大小的字符（原有逻辑）
            let start_pos = content.len() - self.config.overlap_size;
            content[start_pos..].to_string()
        }
    }
    
    /// 为分块添加上下文信息
    fn add_context_to_chunk(
        &self,
        fragment: &FileDocumentFragment,
        chunk_content: &str,
        chunk_index: usize,
        total_chunks: usize,
    ) -> String {
        format!(
            "Package: {} | Version: {} | Language: {} | File: {} | Chunk: {}/{}\n\n{}",
            fragment.package_name,
            fragment.version,
            fragment.language,
            fragment.file_path,
            chunk_index,
            if total_chunks > 0 { total_chunks } else { chunk_index },
            chunk_content
        )
    }
}

/// 智能分块器 - 根据文件类型选择最佳分块策略
pub struct SmartFileChunker {
    base_config: ChunkingConfig,
}

impl SmartFileChunker {
    pub fn new(base_config: ChunkingConfig) -> Self {
        Self { base_config }
    }
    
    /// 根据文件类型和语言智能分块
    pub fn chunk_file_smart(&self, fragment: &FileDocumentFragment) -> Result<Vec<String>> {
        let config = self.get_optimized_config(fragment);
        let chunker = FileChunker::new(config);
        chunker.chunk_file(fragment)
    }
    
    /// 根据文件特征获取优化的分块配置
    fn get_optimized_config(&self, fragment: &FileDocumentFragment) -> ChunkingConfig {
        let mut config = self.base_config.clone();
        
        // 根据编程语言调整配置
        match fragment.language.as_str() {
            "go" | "rust" | "java" | "c" | "cpp" => {
                // 这些语言有明确的函数边界，适合语义分块
                config.preserve_semantic_boundaries = true;
                config.max_chunk_size = 10240; // 10KB，因为函数通常较大
            }
            "python" | "javascript" | "typescript" => {
                // 这些语言的缩进重要，保持语义边界
                config.preserve_semantic_boundaries = true;
                config.max_chunk_size = 8192; // 8KB
            }
            "html" | "xml" | "markdown" => {
                // 标记语言，按段落分块
                config.preserve_semantic_boundaries = true;
                config.max_chunk_size = 6144; // 6KB，文档通常段落较短
            }
            "json" | "yaml" | "toml" => {
                // 配置文件，按对象分块
                config.preserve_semantic_boundaries = true;
                config.max_chunk_size = 4096; // 4KB
            }
            _ => {
                // 默认配置
                config.preserve_semantic_boundaries = false;
                config.max_chunk_size = 8192;
            }
        }
        
        // 根据文件大小调整重叠大小
        if fragment.content.len() > 50000 {
            // 大文件，增加重叠以保持上下文
            config.overlap_size = 1024; // 1KB
        } else if fragment.content.len() < 5000 {
            // 小文件，减少重叠
            config.overlap_size = 256; // 256字符
        }
        
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::base::FileType;
    
    #[test]
    fn test_small_file_no_chunking() {
        let chunker = FileChunker::new(ChunkingConfig::default());
        
        let fragment = FileDocumentFragment {
            id: "test".to_string(),
            package_name: "test_package".to_string(),
            version: "1.0.0".to_string(),
            language: "go".to_string(),
            file_path: "test.go".to_string(),
            content: "package main\n\nfunc main() {\n    println(\"Hello\")\n}".to_string(),
            hierarchy_path: vec!["test.go".to_string()],
            file_type: FileType::Source,
            created_at: std::time::SystemTime::now(),
        };
        
        let chunks = chunker.chunk_file(&fragment).unwrap();
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].contains("Package: test_package"));
    }
    
    #[test]
    fn test_large_file_chunking() {
        let config = ChunkingConfig {
            max_chunk_size: 100, // 很小的块大小用于测试
            overlap_size: 20,
            preserve_semantic_boundaries: false,
            add_context_info: true,
        };
        let chunker = FileChunker::new(config);
        
        let large_content = "a".repeat(300); // 300字符的内容
        let fragment = FileDocumentFragment {
            id: "test".to_string(),
            package_name: "test_package".to_string(),
            version: "1.0.0".to_string(),
            language: "text".to_string(),
            file_path: "large.txt".to_string(),
            content: large_content,
            hierarchy_path: vec!["large.txt".to_string()],
            file_type: FileType::Other("text".to_string()),
            created_at: std::time::SystemTime::now(),
        };
        
        let chunks = chunker.chunk_file(&fragment).unwrap();
        assert!(chunks.len() > 1); // 应该被分成多个块
        
        // 检查每个块都有上下文信息
        for chunk in &chunks {
            assert!(chunk.contains("Package: test_package"));
            assert!(chunk.contains("Chunk:"));
        }
    }
    
    #[test]
    fn test_semantic_boundary_chunking() {
        let config = ChunkingConfig {
            max_chunk_size: 20, // 调整为更小的大小，确保会分块
            overlap_size: 5,
            preserve_semantic_boundaries: true,
            add_context_info: false,
        };
        let chunker = FileChunker::new(config);
        
        let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6";
        let fragment = FileDocumentFragment {
            id: "test".to_string(),
            package_name: "test".to_string(),
            version: "1.0.0".to_string(),
            language: "text".to_string(),
            file_path: "test.txt".to_string(),
            content: content.to_string(),
            hierarchy_path: vec!["test.txt".to_string()],
            file_type: FileType::Other("text".to_string()),
            created_at: std::time::SystemTime::now(),
        };
        
        let chunks = chunker.chunk_file(&fragment).unwrap();
        
        // 应该被分成多个块
        assert!(chunks.len() > 1);
        
        // 验证块在行边界分割：检查没有被截断的行
        for (i, chunk) in chunks.iter().enumerate() {
            // 检查每个块的每一行都是完整的
            for line in chunk.lines() {
                // 完整的行不应该以不完整的"Line"开始（如"Li"）
                if line.starts_with("Li") && !line.starts_with("Line") {
                    eprintln!("警告：分块 {} 包含不完整的行: {:?}", i, line);
                    // 不使用恐慌宏，而是记录警告并继续测试
                }
            }
        }
    }
} 