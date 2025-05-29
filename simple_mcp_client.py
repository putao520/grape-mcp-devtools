#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
🤖 简易MCP客户端 - 用于测试Grape MCP DevTools
"""

import asyncio
import json
import subprocess
import signal
import sys
import os
import uuid
import time
from typing import Dict, List, Optional, Any

# 设置Windows控制台编码
if sys.platform == "win32":
    import locale
    try:
        # 尝试设置UTF-8编码
        os.system("chcp 65001 > nul")
        sys.stdout.reconfigure(encoding='utf-8')
        sys.stderr.reconfigure(encoding='utf-8')
    except:
        pass

try:
    from rich.console import Console
    from rich.panel import Panel
    from rich.table import Table
    from rich.prompt import Prompt
    HAS_RICH = True
    # 为Windows设置控制台
    console = Console(force_terminal=True, legacy_windows=False)
except ImportError:
    HAS_RICH = False
    print("💡 提示: 安装 'rich' 库可获得更好的显示效果: pip install rich")

if HAS_RICH:
    def print_info(msg: str): console.print(msg, style="blue")
    def print_success(msg: str): console.print(msg, style="green")
    def print_error(msg: str): console.print(msg, style="red")
    def print_warning(msg: str): console.print(msg, style="yellow")
else:
    def print_info(msg: str): print(f"[INFO] {msg}")
    def print_success(msg: str): print(f"[SUCCESS] {msg}")
    def print_error(msg: str): print(f"[ERROR] {msg}")
    def print_warning(msg: str): print(f"[WARNING] {msg}")

class SimpleMCPClient:
    """简易MCP客户端"""
    
    def __init__(self):
        self.server_process: Optional[subprocess.Popen] = None
        self.tools: List[Dict] = []
        self.external_server = False  # 简化：直接启动自己的服务器
    
    async def start_server(self, server_command: List[str] = None) -> bool:
        """启动MCP服务器"""
        try:
            print_info("🚀 启动MCP服务器...")
            
            if not server_command:
                server_command = ["cargo", "run", "--bin", "grape-mcp-devtools"]
            
            self.server_process = subprocess.Popen(
                server_command,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=0
            )
            
            # 等待服务器启动
            await asyncio.sleep(3)
            
            if self.server_process.poll() is None:
                print_success("✅ MCP服务器启动成功")
                return True
            else:
                print_error("❌ MCP服务器启动失败")
                return False
                
        except Exception as e:
            print_error(f"❌ 启动服务器失败: {e}")
            return False
    
    async def stop_server(self):
        """停止MCP服务器"""
        if self.server_process:
            self.server_process.terminate()
            try:
                self.server_process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.server_process.kill()
            
            print_warning("👋 MCP服务器已关闭")
    
    async def send_request(self, method: str, params: Dict[str, Any] = None) -> Optional[Dict]:
        """发送MCP请求"""
        if not self.server_process:
            print_error("MCP服务器未连接")
            return None
        
        request = {
            "jsonrpc": "2.0",
            "version": "2025-03-26",
            "id": str(uuid.uuid4()),
            "method": method,
            "params": params or {}
        }
        
        request_json = json.dumps(request) + "\n"
        
        try:
            print_info(f"📤 发送请求: {method}")
            self.server_process.stdin.write(request_json)
            self.server_process.stdin.flush()
            
            # 读取响应 - 使用超时机制
            print_info("📥 等待响应...")
            
            # 使用select或者poll来检查是否有数据可读
            import select
            import sys
            
            if sys.platform == "win32":
                # Windows下使用简单的超时等待
                import time
                start_time = time.time()
                timeout = 10  # 10秒超时
                
                while time.time() - start_time < timeout:
                    if self.server_process.poll() is not None:
                        print_error("服务器进程已退出")
                        return None
                    
                    # 尝试读取一行
                    try:
                        # 设置非阻塞模式
                        import os
                        import fcntl
                        # Windows下无法使用fcntl，使用其他方法
                        response_line = self.server_process.stdout.readline()
                        if response_line:
                            break
                    except:
                        pass
                    
                    time.sleep(0.1)  # 短暂等待
                else:
                    print_error("响应超时")
                    return None
            else:
                # Unix系统使用select
                ready, _, _ = select.select([self.server_process.stdout], [], [], 10)
                if not ready:
                    print_error("响应超时")
                    return None
                response_line = self.server_process.stdout.readline()
            
            if not response_line:
                print_error("服务器无响应")
                return None
            
            print_info(f"📋 收到响应: {response_line.strip()[:100]}...")
            response_data = json.loads(response_line.strip())
            
            if "error" in response_data and response_data["error"]:
                print_error(f"服务器错误: {response_data['error']}")
                return None
            
            return response_data.get("result")
            
        except Exception as e:
            print_error(f"MCP通信失败: {e}")
            return None
    
    async def initialize(self) -> bool:
        """初始化MCP连接"""
        print_info("🔧 初始化MCP连接...")
        params = {
            "client_name": "simple-mcp-client",
            "client_version": "1.0.0",
            "capabilities": ["documentSearch", "versionInfo"]
        }
        
        result = await self.send_request("initialize", params)
        if result:
            print_success("✅ MCP连接初始化成功")
            return True
        else:
            print_error("❌ 初始化失败")
            return False
    
    async def list_tools(self) -> bool:
        """获取工具列表"""
        print_info("📚 获取工具列表...")
        result = await self.send_request("tools/list")
        if result and "tools" in result:
            self.tools = result["tools"]
            
            if HAS_RICH:
                table = Table(title="📚 可用工具列表")
                table.add_column("工具名称", style="cyan")
                table.add_column("描述", style="green")
                
                for tool in self.tools:
                    table.add_row(tool["name"], tool["description"])
                
                console.print(table)
            else:
                print("📚 可用工具列表:")
                for i, tool in enumerate(self.tools, 1):
                    print(f"  {i}. {tool['name']}: {tool['description']}")
            
            return True
        else:
            print_error("❌ 获取工具列表失败")
            return False
    
    async def call_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Optional[Dict]:
        """调用MCP工具"""
        params = {
            "name": tool_name,
            "arguments": arguments
        }
        
        result = await self.send_request("tools/call", params)
        return result
    
    def show_tools(self):
        """显示可用工具"""
        if not self.tools:
            print_warning("⚠️  没有可用工具")
            return
        
        print("📚 可用工具:")
        for i, tool in enumerate(self.tools, 1):
            print(f"  {i}. {tool['name']}: {tool['description']}")

async def run_interactive_test():
    """运行交互式测试"""
    client = SimpleMCPClient()
    
    # 启动MCP服务器
    if not await client.start_server():
        return
    
    try:
        # 初始化连接
        if not await client.initialize():
            return
        
        # 获取工具列表
        if not await client.list_tools():
            return
        
        print("\n🎯 交互式测试模式已启动！")
        print("💡 输入工具名称来调用工具，或输入 'quit' 退出")
        print("💡 可用命令: list, test, quit\n")
        
        while True:
            try:
                if HAS_RICH:
                    user_input = Prompt.ask("🤔 [bold blue]请输入命令[/bold blue]")
                else:
                    user_input = input("🤔 请输入命令: ").strip()
                
                if user_input.lower() in ['quit', 'exit', 'bye', '退出']:
                    break
                elif user_input.lower() == 'list':
                    client.show_tools()
                elif user_input.lower() == 'test':
                    await run_predefined_tests(client)
                elif user_input.startswith('search_docs'):
                    await test_search_docs(client)
                elif user_input.startswith('check_version'):
                    await test_check_version(client)
                elif user_input.startswith('get_api_docs'):
                    await test_get_api_docs(client)
                else:
                    print_warning(f"未知命令: {user_input}")
                    print("💡 可用命令: list, test, search_docs, check_version, get_api_docs, quit")
                
            except KeyboardInterrupt:
                break
            except EOFError:
                break
    
    finally:
        await client.stop_server()

async def test_search_docs(client: SimpleMCPClient):
    """测试搜索文档工具"""
    print_info("🔍 测试搜索文档工具...")
    
    test_cases = [
        {"query": "http client", "language": "rust", "limit": 3},
        {"query": "web framework", "language": "python", "limit": 2},
        {"query": "json parsing", "language": "javascript", "limit": 3}
    ]
    
    for i, args in enumerate(test_cases, 1):
        print(f"\n📋 测试案例 {i}: {args}")
        result = await client.call_tool("search_docs", args)
        if result:
            print_success("✅ 调用成功")
            print(f"📄 结果: {json.dumps(result, ensure_ascii=False, indent=2)[:200]}...")
        else:
            print_error("❌ 调用失败")

async def test_check_version(client: SimpleMCPClient):
    """测试版本检查工具"""
    print_info("📦 测试版本检查工具...")
    
    test_cases = [
        {"package": "serde", "language": "rust"},
        {"package": "requests", "language": "python"},
        {"package": "express", "language": "javascript"}
    ]
    
    for i, args in enumerate(test_cases, 1):
        print(f"\n📋 测试案例 {i}: {args}")
        result = await client.call_tool("check_version", args)
        if result:
            print_success("✅ 调用成功")
            print(f"📄 结果: {json.dumps(result, ensure_ascii=False, indent=2)[:200]}...")
        else:
            print_error("❌ 调用失败")

async def test_get_api_docs(client: SimpleMCPClient):
    """测试API文档工具"""
    print_info("📚 测试API文档工具...")
    
    test_cases = [
        {"language": "rust", "package": "std", "query": "Vec"},
        {"language": "python", "package": "requests", "query": "get"},
        {"language": "javascript", "package": "lodash", "query": "map"}
    ]
    
    for i, args in enumerate(test_cases, 1):
        print(f"\n📋 测试案例 {i}: {args}")
        result = await client.call_tool("get_api_docs", args)
        if result:
            print_success("✅ 调用成功")
            print(f"📄 结果: {json.dumps(result, ensure_ascii=False, indent=2)[:200]}...")
        else:
            print_error("❌ 调用失败")

async def run_predefined_tests(client: SimpleMCPClient):
    """运行预定义测试"""
    print_info("🧪 运行预定义测试套件...")
    
    await test_search_docs(client)
    await test_check_version(client)
    await test_get_api_docs(client)
    
    print_success("🎉 所有测试完成！")

async def run_simple_test():
    """运行简单测试"""
    client = SimpleMCPClient()
    
    # 启动MCP服务器
    if not await client.start_server():
        return
    
    try:
        # 初始化连接
        if not await client.initialize():
            return
        
        # 获取工具列表
        if not await client.list_tools():
            return
        
        # 运行预定义测试
        await run_predefined_tests(client)
    
    finally:
        await client.stop_server()

def show_usage():
    """显示使用说明"""
    print("""
🤖 简易MCP客户端使用说明

用法:
    python simple_mcp_client.py [command]

命令:
    test        - 运行简单测试
    interactive - 运行交互式测试 (默认)
    help        - 显示此帮助信息

环境要求:
    - Python 3.7+
    - Rust 工具链 (用于编译MCP服务器)
    - 可选: rich 库 (pip install rich) 用于更好的显示效果

MCP服务器管理:
    - 启动服务器: .\\start_mcp_server.ps1 start 或 mcp start
    - 检查状态: .\\start_mcp_server.ps1 status 或 mcp status  
    - 停止服务器: .\\start_mcp_server.ps1 stop 或 mcp stop

示例:
    mcp start                                 # 启动MCP服务器
    python simple_mcp_client.py test         # 运行自动测试
    python simple_mcp_client.py interactive  # 运行交互式测试
    mcp stop                                  # 停止MCP服务器
    """)

def main():
    """主函数"""
    # 设置信号处理
    def signal_handler(sig, frame):
        print("\n👋 再见！")
        sys.exit(0)
    
    signal.signal(signal.SIGINT, signal_handler)
    
    # 显示欢迎信息
    if HAS_RICH:
        console.print(Panel.fit(
            "[bold blue]🤖 简易MCP客户端[/bold blue]\n"
            "[dim]测试Grape MCP DevTools服务器[/dim]",
            border_style="blue"
        ))
    else:
        print("🤖 简易MCP客户端")
        print("=" * 40)
        print("测试Grape MCP DevTools服务器")
        print("=" * 40)
    
    # 解析命令行参数
    command = sys.argv[1] if len(sys.argv) > 1 else "interactive"
    
    if command == "help":
        show_usage()
    elif command == "test":
        asyncio.run(run_simple_test())
    elif command == "interactive":
        asyncio.run(run_interactive_test())
    else:
        print_error(f"未知命令: {command}")
        show_usage()

if __name__ == "__main__":
    main() 