#!/usr/bin/env python3
"""
🔧 MCP客户端环境设置脚本
自动安装依赖并配置环境
"""

import os
import sys
import subprocess
import platform
from pathlib import Path

def run_command(cmd, description=""):
    """运行命令并显示结果"""
    print(f"🔧 {description}...")
    try:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        if result.returncode == 0:
            print(f"✅ {description} 成功")
            return True
        else:
            print(f"❌ {description} 失败:")
            print(result.stderr)
            return False
    except Exception as e:
        print(f"❌ {description} 失败: {e}")
        return False

def check_python_version():
    """检查Python版本"""
    version = sys.version_info
    if version.major >= 3 and version.minor >= 7:
        print(f"✅ Python版本: {version.major}.{version.minor}.{version.micro}")
        return True
    else:
        print(f"❌ Python版本过低: {version.major}.{version.minor}.{version.micro}")
        print("需要Python 3.7+")
        return False

def check_rust():
    """检查Rust工具链"""
    return run_command("cargo --version", "检查Rust工具链")

def install_python_deps():
    """安装Python依赖"""
    deps = [
        "rich",
        "httpx", 
        "python-dotenv",
        "click",
        "asyncio-subprocess"
    ]
    
    for dep in deps:
        if not run_command(f"pip install {dep}", f"安装 {dep}"):
            print(f"⚠️ 安装 {dep} 失败，可能需要手动安装")

def create_env_file():
    """创建示例.env文件"""
    env_content = """# MCP客户端环境配置示例

# LLM配置 (用于AI对话功能)
LLM_API_BASE_URL=https://integrate.api.nvidia.com/v1
LLM_API_KEY=your-llm-api-key-here
LLM_MODEL_NAME=nvidia/llama-3.1-nemotron-70b-instruct

# Embedding配置 (用于向量化功能)
EMBEDDING_API_BASE_URL=https://integrate.api.nvidia.com/v1
EMBEDDING_API_KEY=your-embedding-api-key-here
EMBEDDING_MODEL_NAME=nvidia/nv-embedqa-mistral-7b-v2

# 向量化参数
VECTOR_DIMENSION=768
CHUNK_SIZE=8192
CHUNK_OVERLAP=512
MAX_FILE_SIZE=1048576
MAX_CONCURRENT_FILES=10
VECTORIZATION_TIMEOUT_SECS=30
EMBEDDING_TIMEOUT_SECS=30
"""
    
    if not Path(".env").exists():
        with open(".env", "w", encoding="utf-8") as f:
            f.write(env_content)
        print("✅ 创建了示例.env文件")
        print("💡 请编辑.env文件设置你的API密钥")
    else:
        print("⚠️ .env文件已存在，跳过创建")

def create_test_script():
    """创建快速测试脚本"""
    test_script = """#!/usr/bin/env python3
# 快速测试MCP通信
import asyncio
import sys
import os
sys.path.append(os.path.dirname(__file__))

from simple_mcp_client import run_simple_test

if __name__ == "__main__":
    print("🧪 快速MCP通信测试")
    asyncio.run(run_simple_test())
"""
    
    with open("quick_test.py", "w", encoding="utf-8") as f:
        f.write(test_script)
    
    # 在Unix系统上设置执行权限
    if platform.system() != "Windows":
        os.chmod("quick_test.py", 0o755)
    
    print("✅ 创建了快速测试脚本: quick_test.py")

def display_usage_guide():
    """显示使用指南"""
    print("\n" + "="*60)
    print("🎉 MCP客户端环境设置完成！")
    print("="*60)
    print()
    print("📚 使用指南:")
    print()
    print("1. 基础测试:")
    print("   python simple_mcp_client.py test")
    print()
    print("2. 交互式测试:")
    print("   python simple_mcp_client.py interactive")
    print()
    print("3. 快速测试:")
    print("   python quick_test.py")
    print()
    print("4. 智能对话 (需要配置LLM API):")
    print("   python mcp_client.py chat")
    print()
    print("💡 配置提示:")
    print("- 编辑 .env 文件设置API密钥")
    print("- 确保MCP服务器能正常编译: cargo check")
    print("- 如需向量化功能，配置EMBEDDING_API_KEY")
    print("- 如需AI对话功能，配置LLM_API_KEY")
    print()
    print("🔗 相关文件:")
    print("- simple_mcp_client.py: 简易MCP客户端")
    print("- mcp_client.py: 智能MCP客户端 (含LLM功能)")
    print("- quick_test.py: 快速测试脚本")
    print("- .env: 环境变量配置")
    print()

def main():
    """主函数"""
    print("🚀 MCP客户端环境设置")
    print("="*40)
    
    # 检查Python版本
    if not check_python_version():
        return
    
    # 检查Rust工具链
    if not check_rust():
        print("⚠️ 未检测到Rust工具链，请先安装Rust")
        print("💡 访问 https://rustup.rs/ 安装Rust")
    
    # 安装Python依赖
    install_python_deps()
    
    # 创建配置文件
    create_env_file()
    
    # 创建测试脚本
    create_test_script()
    
    # 显示使用指南
    display_usage_guide()

if __name__ == "__main__":
    main() 