use std::collections::HashMap;

/// 构建词频向量
fn build_word_frequency_vector(text: &str) -> HashMap<String, f32> {
    let mut word_freq = HashMap::new();
    let words: Vec<&str> = text.split_whitespace().collect();
    let total_words = words.len() as f32;
    
    if total_words == 0.0 {
        return word_freq;
    }
    
    // 计算词频
    for word in words {
        let word_lower = word.to_lowercase();
        // 过滤掉过短的词和常见停用词
        if word_lower.len() >= 2 && !is_stop_word(&word_lower) {
            *word_freq.entry(word_lower).or_insert(0.0) += 1.0;
        }
    }
    
    // 标准化词频
    for freq in word_freq.values_mut() {
        *freq /= total_words;
    }
    
    word_freq
}

/// 计算两个词频向量的余弦相似度
fn calculate_cosine_similarity(vector1: &HashMap<String, f32>, vector2: &HashMap<String, f32>) -> f32 {
    if vector1.is_empty() && vector2.is_empty() {
        return 1.0;
    }
    if vector1.is_empty() || vector2.is_empty() {
        return 0.0;
    }
    
    // 获取所有唯一词汇
    let mut all_words: std::collections::HashSet<String> = std::collections::HashSet::new();
    all_words.extend(vector1.keys().cloned());
    all_words.extend(vector2.keys().cloned());
    
    // 计算点积和向量模长
    let mut dot_product = 0.0;
    let mut norm1 = 0.0;
    let mut norm2 = 0.0;
    
    for word in all_words {
        let freq1 = vector1.get(&word).unwrap_or(&0.0);
        let freq2 = vector2.get(&word).unwrap_or(&0.0);
        
        dot_product += freq1 * freq2;
        norm1 += freq1 * freq1;
        norm2 += freq2 * freq2;
    }
    
    // 计算余弦相似度
    let norm_product = norm1.sqrt() * norm2.sqrt();
    if norm_product == 0.0 {
        return 0.0;
    }
    
    (dot_product / norm_product).max(0.0).min(1.0)
}

/// 判断是否为停用词
fn is_stop_word(word: &str) -> bool {
    const STOP_WORDS: &[&str] = &[
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
        "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do", "does", "did",
        "will", "would", "could", "should", "may", "might", "can", "this", "that", "these", "those",
        "i", "you", "he", "she", "it", "we", "they", "me", "him", "her", "us", "them", "my", "your",
        "his", "her", "its", "our", "their", "from", "up", "about", "into", "through", "during",
        "before", "after", "above", "below", "between", "among", "within", "without", "under", "over"
    ];
    
    STOP_WORDS.contains(&word)
}

/// 计算文本相似度（基于余弦相似度算法）
fn calculate_text_similarity(text1: &str, text2: &str) -> f32 {
    if text1.is_empty() && text2.is_empty() {
        return 1.0;
    }
    if text1.is_empty() || text2.is_empty() {
        return 0.0;
    }
    
    // 构建词频向量
    let vector1 = build_word_frequency_vector(text1);
    let vector2 = build_word_frequency_vector(text2);
    
    // 计算余弦相似度
    calculate_cosine_similarity(&vector1, &vector2)
}

fn main() {
    println!("🧮 余弦相似度算法测试");
    println!("{}", "=".repeat(50));
    
    // 测试1: 完全相同的内容
    let content1 = "This is a test document about Rust programming.";
    let content2 = "This is a test document about Rust programming.";
    let similarity1 = calculate_text_similarity(content1, content2);
    println!("✅ 测试1 - 完全相同内容:");
    println!("   文本1: {}", content1);
    println!("   文本2: {}", content2);
    println!("   相似度: {:.4} (期望: >0.95)", similarity1);
    assert!(similarity1 > 0.95, "完全相同的内容相似度应该很高");
    
    // 测试2: 相似但不完全相同的内容
    let content3 = "This is a test document about Rust programming language.";
    let content4 = "This is a test document about Rust programming and development.";
    let similarity2 = calculate_text_similarity(content3, content4);
    println!("\n✅ 测试2 - 相似内容:");
    println!("   文本1: {}", content3);
    println!("   文本2: {}", content4);
    println!("   相似度: {:.4} (期望: 0.7-0.95)", similarity2);
    assert!(similarity2 > 0.7 && similarity2 < 0.95, "相似内容相似度应该在70%-95%之间");
    
    // 测试3: 完全不同的内容
    let content5 = "This is about Rust programming.";
    let content6 = "This is about Python web development.";
    let similarity3 = calculate_text_similarity(content5, content6);
    println!("\n✅ 测试3 - 不同内容:");
    println!("   文本1: {}", content5);
    println!("   文本2: {}", content6);
    println!("   相似度: {:.4} (期望: <0.7)", similarity3);
    assert!(similarity3 < 0.7, "不同内容相似度应该较低");
    
    // 测试4: 空文本处理
    let empty_similarity = calculate_text_similarity("", "");
    println!("\n✅ 测试4 - 空文本:");
    println!("   相似度: {:.4} (期望: 1.0)", empty_similarity);
    assert_eq!(empty_similarity, 1.0, "空文本相似度应该为1.0");
    
    let mixed_similarity = calculate_text_similarity("test", "");
    println!("   混合空文本相似度: {:.4} (期望: 0.0)", mixed_similarity);
    assert_eq!(mixed_similarity, 0.0, "空文本与非空文本相似度应该为0.0");
    
    // 测试5: 词频向量测试
    println!("\n🔍 词频向量测试:");
    let test_text = "rust programming rust language programming tutorial";
    let vector = build_word_frequency_vector(test_text);
    println!("   文本: {}", test_text);
    println!("   词频向量:");
    for (word, freq) in &vector {
        println!("     {}: {:.4}", word, freq);
    }
    
    // 验证词频计算
    assert!(vector.contains_key("rust"), "应该包含'rust'");
    assert!(vector.contains_key("programming"), "应该包含'programming'");
    let rust_freq = vector.get("rust").unwrap();
    let programming_freq = vector.get("programming").unwrap();
    assert!((rust_freq - programming_freq).abs() < 0.001, "'rust'和'programming'出现频率应该相同: rust={:.4}, programming={:.4}", rust_freq, programming_freq);
    
    // 测试6: 停用词过滤测试
    println!("\n🚫 停用词过滤测试:");
    let text_with_stopwords = "the rust programming language is very powerful and safe";
    let filtered_vector = build_word_frequency_vector(text_with_stopwords);
    println!("   原文本: {}", text_with_stopwords);
    println!("   过滤后词频向量:");
    for (word, freq) in &filtered_vector {
        println!("     {}: {:.4}", word, freq);
    }
    
    // 验证停用词被过滤
    assert!(!filtered_vector.contains_key("the"), "停用词'the'应该被过滤");
    assert!(!filtered_vector.contains_key("is"), "停用词'is'应该被过滤");
    assert!(!filtered_vector.contains_key("and"), "停用词'and'应该被过滤");
    assert!(filtered_vector.contains_key("rust"), "关键词'rust'应该保留");
    assert!(filtered_vector.contains_key("programming"), "关键词'programming'应该保留");
    
    // 测试7: 余弦相似度计算测试
    println!("\n📐 余弦相似度计算测试:");
    let mut vector1 = HashMap::new();
    vector1.insert("rust".to_string(), 0.5);
    vector1.insert("programming".to_string(), 0.3);
    vector1.insert("language".to_string(), 0.2);
    
    let mut vector2 = HashMap::new();
    vector2.insert("rust".to_string(), 0.4);
    vector2.insert("programming".to_string(), 0.4);
    vector2.insert("tutorial".to_string(), 0.2);
    
    let cosine_sim = calculate_cosine_similarity(&vector1, &vector2);
    println!("   向量1: {:?}", vector1);
    println!("   向量2: {:?}", vector2);
    println!("   余弦相似度: {:.4}", cosine_sim);
    assert!(cosine_sim > 0.0 && cosine_sim <= 1.0, "余弦相似度应该在[0,1]范围内");
    assert!(cosine_sim > 0.5, "有共同词汇的向量相似度应该较高");
    
    // 测试相同向量
    let identical_sim = calculate_cosine_similarity(&vector1, &vector1);
    println!("   相同向量相似度: {:.4} (期望: 1.0)", identical_sim);
    assert!((identical_sim - 1.0).abs() < 0.001, "相同向量的余弦相似度应该为1.0");
    
    println!("\n🎉 所有余弦相似度测试通过！");
    println!("✨ 余弦相似度算法实现正确，可以用于智能文档重复检测");
} 