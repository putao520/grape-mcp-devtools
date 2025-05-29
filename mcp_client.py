#!/usr/bin/env python3
"""
🤖 Grape MCP DevTools 智能客户端
一个支持MCP协议的AI助手客户端，可以与MCP服务器通信并提供智能对话
"""

import asyncio
import json
import os
import sys
import subprocess
import signal
import uuid
from typing import Dict, List, Optional, Any, Tuple
from dataclasses import dataclass, asdict
from pathlib import Path

import click
import httpx
from rich.console import Console
from rich.panel import Panel
from rich.table import Table
from rich.markdown import Markdown
from rich.progress import Progress, SpinnerColumn, TextColumn
from rich.prompt import Prompt
from dotenv import load_dotenv

# 加载环境变量
load_dotenv()

console = Console()

@dataclass
class MCPRequest:
    """MCP请求结构"""
    jsonrpc: str = "2.0"
    version: str = "2025-03-26"
    id: str = ""
    method: str = ""
    params: Dict[str, Any] = None
    
    def __post_init__(self):
        if not self.id:
            self.id = str(uuid.uuid4())
        if self.params is None:
            self.params = {}

@dataclass
class MCPResponse:
    """MCP响应结构"""
    jsonrpc: str
    version: str
    id: str
    result: Optional[Dict[str, Any]] = None
    error: Optional[Dict[str, Any]] = None

class LLMClient:
    """LLM客户端，支持多种API提供商"""
    
    def __init__(self):
        self.client = httpx.AsyncClient(timeout=30.0)
        self.api_base_url = os.getenv("LLM_API_BASE_URL", "https://integrate.api.nvidia.com/v1")
        self.api_key = os.getenv("LLM_API_KEY")
        self.model_name = os.getenv("LLM_MODEL_NAME", "nvidia/llama-3.1-nemotron-70b-instruct")
        
        if not self.api_key:
            console.print("⚠️  警告: 未设置LLM_API_KEY，LLM功能将不可用", style="yellow")
    
    async def chat_completion(self, messages: List[Dict[str, str]], tools: Optional[List[Dict]] = None) -> str:
        """发送聊天完成请求"""
        if not self.api_key:
            return "❌ LLM API密钥未配置，无法使用LLM功能"
        
        try:
            payload = {
                "model": self.model_name,
                "messages": messages,
                "max_tokens": 1024,
                "temperature": 0.7
            }
            
            if tools:
                payload["tools"] = tools
                payload["tool_choice"] = "auto"
            
            response = await self.client.post(
                f"{self.api_base_url}/chat/completions",
                headers={
                    "Authorization": f"Bearer {self.api_key}",
                    "Content-Type": "application/json"
                },
                json=payload
            )
            
            if response.status_code != 200:
                return f"❌ LLM API请求失败: {response.status_code} - {response.text}"
            
            response_data = response.json()
            
            if "choices" in response_data and len(response_data["choices"]) > 0:
                choice = response_data["choices"][0]
                
                # 检查是否有工具调用
                if "tool_calls" in choice.get("message", {}):
                    return choice["message"]
                else:
                    return choice["message"]["content"]
            
            return "❌ LLM响应格式错误"
            
        except Exception as e:
            return f"❌ LLM请求失败: {e}"

class MCPClient:
    """MCP客户端"""
    
    def __init__(self):
        self.server_process: Optional[subprocess.Popen] = None
        self.tools: List[Dict] = []
        self.llm_client = LLMClient()
        self.conversation_history: List[Dict[str, str]] = []
        
    async def start_server(self, server_command: List[str]) -> bool:
        """启动MCP服务器"""
        try:
            console.print("🚀 启动MCP服务器...", style="blue")
            
            self.server_process = subprocess.Popen(
                server_command,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=0  # 无缓冲
            )
            
            # 等待服务器启动
            await asyncio.sleep(2)
            
            if self.server_process.poll() is None:
                console.print("✅ MCP服务器启动成功", style="green")
                return True
            else:
                console.print("❌ MCP服务器启动失败", style="red")
                return False
                
        except Exception as e:
            console.print(f"❌ 启动服务器失败: {e}", style="red")
            return False
    
    async def stop_server(self):
        """停止MCP服务器"""
        if self.server_process:
            self.server_process.terminate()
            self.server_process.wait()
            console.print("👋 MCP服务器已关闭", style="yellow")
    
    async def send_request(self, request: MCPRequest) -> MCPResponse:
        """发送MCP请求"""
        if not self.server_process:
            raise Exception("MCP服务器未启动")
        
        request_json = json.dumps(asdict(request)) + "\n"
        
        try:
            self.server_process.stdin.write(request_json)
            self.server_process.stdin.flush()
            
            # 读取响应
            response_line = self.server_process.stdout.readline()
            if not response_line:
                raise Exception("服务器无响应")
            
            response_data = json.loads(response_line.strip())
            return MCPResponse(**response_data)
            
        except Exception as e:
            raise Exception(f"MCP通信失败: {e}")
    
    async def initialize(self) -> bool:
        """初始化MCP连接"""
        request = MCPRequest(
            method="initialize",
            params={
                "client_name": "grape-mcp-client",
                "client_version": "1.0.0",
                "capabilities": ["documentSearch", "versionInfo"]
            }
        )
        
        try:
            response = await self.send_request(request)
            if response.error:
                console.print(f"❌ 初始化失败: {response.error}", style="red")
                return False
            
            console.print("✅ MCP连接初始化成功", style="green")
            return True
            
        except Exception as e:
            console.print(f"❌ 初始化失败: {e}", style="red")
            return False
    
    async def list_tools(self) -> bool:
        """获取工具列表"""
        request = MCPRequest(method="tools/list")
        
        try:
            response = await self.send_request(request)
            if response.error:
                console.print(f"❌ 获取工具列表失败: {response.error}", style="red")
                return False
            
            self.tools = response.result.get("tools", [])
            
            # 显示工具列表
            table = Table(title="📚 可用工具列表")
            table.add_column("工具名称", style="cyan")
            table.add_column("描述", style="green")
            table.add_column("参数", style="yellow")
            
            for tool in self.tools:
                params_str = ", ".join(tool.get("inputSchema", {}).get("properties", {}).keys())
                table.add_row(
                    tool["name"],
                    tool["description"],
                    params_str[:50] + "..." if len(params_str) > 50 else params_str
                )
            
            console.print(table)
            return True
            
        except Exception as e:
            console.print(f"❌ 获取工具列表失败: {e}", style="red")
            return False
    
    async def call_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Optional[Dict]:
        """调用MCP工具"""
        request = MCPRequest(
            method="tools/call",
            params={
                "name": tool_name,
                "arguments": arguments
            }
        )
        
        try:
            response = await self.send_request(request)
            if response.error:
                console.print(f"❌ 工具调用失败: {response.error}", style="red")
                return None
            
            return response.result
            
        except Exception as e:
            console.print(f"❌ 工具调用失败: {e}", style="red")
            return None
    
    def get_tools_for_llm(self) -> List[Dict]:
        """获取适用于LLM的工具定义"""
        llm_tools = []
        
        for tool in self.tools:
            llm_tool = {
                "type": "function",
                "function": {
                    "name": tool["name"],
                    "description": tool["description"],
                    "parameters": tool.get("inputSchema", {})
                }
            }
            llm_tools.append(llm_tool)
        
        return llm_tools
    
    async def ai_chat(self, user_message: str) -> str:
        """AI对话，自动调用工具"""
        self.conversation_history.append({"role": "user", "content": user_message})
        
        # 构建系统提示
        system_prompt = """你是一个专业的开发工具助手，可以帮助用户查询包信息、文档和版本信息。

可用工具：
- search_docs: 搜索文档和包信息
- check_version: 检查包版本信息
- get_api_docs: 获取API文档
- vector_docs: 向量化文档管理

请根据用户的问题智能选择合适的工具，并以中文回复。如果需要调用工具，请按照JSON格式返回工具调用信息。"""

        messages = [{"role": "system", "content": system_prompt}] + self.conversation_history
        tools = self.get_tools_for_llm()
        
        response = await self.llm_client.chat_completion(messages, tools)
        
        # 检查是否有工具调用
        if isinstance(response, dict) and "tool_calls" in response:
            # 处理工具调用
            tool_results = []
            for tool_call in response["tool_calls"]:
                function = tool_call["function"]
                tool_name = function["name"]
                arguments = json.loads(function["arguments"])
                
                console.print(f"🔧 调用工具: {tool_name}", style="blue")
                console.print(f"📝 参数: {arguments}", style="dim")
                
                result = await self.call_tool(tool_name, arguments)
                if result:
                    tool_results.append(f"工具 {tool_name} 的结果: {json.dumps(result, ensure_ascii=False, indent=2)}")
            
            # 将工具结果发送回LLM生成最终回复
            if tool_results:
                tool_results_text = "\n\n".join(tool_results)
                final_messages = messages + [
                    {"role": "assistant", "content": response.get("content", "")},
                    {"role": "user", "content": f"工具调用结果：\n{tool_results_text}\n\n请基于这些结果给出最终回复。"}
                ]
                final_response = await self.llm_client.chat_completion(final_messages)
                self.conversation_history.append({"role": "assistant", "content": final_response})
                return final_response
        
        # 普通回复
        if isinstance(response, str):
            self.conversation_history.append({"role": "assistant", "content": response})
            return response
        
        return "❌ 处理AI回复时出现错误"

@click.group()
def cli():
    """🤖 Grape MCP DevTools 智能客户端"""
    pass

@cli.command()
@click.option("--server-cmd", default="cargo run --bin grape-mcp-devtools", help="MCP服务器启动命令")
async def chat(server_cmd: str):
    """💬 启动智能对话模式"""
    client = MCPClient()
    
    # 启动MCP服务器
    server_command = server_cmd.split()
    if not await client.start_server(server_command):
        return
    
    try:
        # 初始化连接
        if not await client.initialize():
            return
        
        # 获取工具列表
        if not await client.list_tools():
            return
        
        console.print("\n🎯 智能对话模式已启动！", style="bold green")
        console.print("💡 你可以询问关于包管理、文档查询、版本检查等问题", style="dim")
        console.print("💡 输入 'quit' 或 'exit' 退出\n", style="dim")
        
        while True:
            try:
                user_input = Prompt.ask("🤔 [bold blue]你[/bold blue]")
                
                if user_input.lower() in ['quit', 'exit', 'bye', '退出']:
                    break
                
                with Progress(
                    SpinnerColumn(),
                    TextColumn("[progress.description]{task.description}"),
                    console=console
                ) as progress:
                    task = progress.add_task("🤖 AI正在思考...", total=None)
                    response = await client.ai_chat(user_input)
                
                console.print(f"\n🤖 [bold green]助手[/bold green]: {response}\n")
                
            except KeyboardInterrupt:
                break
            except EOFError:
                break
    
    finally:
        await client.stop_server()

@cli.command()
@click.option("--server-cmd", default="cargo run --bin grape-mcp-devtools", help="MCP服务器启动命令")
async def test(server_cmd: str):
    """🧪 测试MCP连接和工具"""
    client = MCPClient()
    
    # 启动MCP服务器
    server_command = server_cmd.split()
    if not await client.start_server(server_command):
        return
    
    try:
        # 初始化连接
        if not await client.initialize():
            return
        
        # 获取工具列表
        if not await client.list_tools():
            return
        
        # 测试一些基本工具调用
        test_cases = [
            {
                "tool": "search_docs",
                "args": {"query": "http client", "language": "rust", "limit": 3},
                "description": "搜索Rust HTTP客户端文档"
            },
            {
                "tool": "check_version",
                "args": {"package": "serde", "language": "rust"},
                "description": "检查serde包版本"
            }
        ]
        
        console.print("\n🧪 开始工具测试...\n", style="bold blue")
        
        for i, test_case in enumerate(test_cases, 1):
            console.print(f"📋 测试 {i}: {test_case['description']}", style="cyan")
            
            result = await client.call_tool(test_case["tool"], test_case["args"])
            if result:
                console.print("✅ 测试成功", style="green")
                # 显示部分结果
                result_str = json.dumps(result, ensure_ascii=False, indent=2)
                if len(result_str) > 300:
                    result_str = result_str[:300] + "..."
                console.print(Panel(result_str, title="返回结果"))
            else:
                console.print("❌ 测试失败", style="red")
            
            console.print()
    
    finally:
        await client.stop_server()

@cli.command()
@click.option("--tool-name", prompt="工具名称", help="要调用的工具名称")
@click.option("--args", prompt="参数(JSON格式)", help="工具参数，JSON格式")
@click.option("--server-cmd", default="cargo run --bin grape-mcp-devtools", help="MCP服务器启动命令")
async def call(tool_name: str, args: str, server_cmd: str):
    """🔧 直接调用MCP工具"""
    client = MCPClient()
    
    try:
        arguments = json.loads(args)
    except json.JSONDecodeError as e:
        console.print(f"❌ JSON参数格式错误: {e}", style="red")
        return
    
    # 启动MCP服务器
    server_command = server_cmd.split()
    if not await client.start_server(server_command):
        return
    
    try:
        # 初始化连接
        if not await client.initialize():
            return
        
        # 获取工具列表
        if not await client.list_tools():
            return
        
        console.print(f"🔧 调用工具: {tool_name}", style="blue")
        console.print(f"📝 参数: {arguments}", style="dim")
        
        result = await client.call_tool(tool_name, arguments)
        if result:
            console.print("✅ 调用成功", style="green")
            result_json = json.dumps(result, ensure_ascii=False, indent=2)
            console.print(Panel(result_json, title="工具返回结果"))
        else:
            console.print("❌ 调用失败", style="red")
    
    finally:
        await client.stop_server()

def main():
    """主函数"""
    # 设置信号处理
    def signal_handler(sig, frame):
        console.print("\n👋 再见！", style="yellow")
        sys.exit(0)
    
    signal.signal(signal.SIGINT, signal_handler)
    
    # 显示欢迎信息
    console.print(Panel.fit(
        "[bold blue]🤖 Grape MCP DevTools 智能客户端[/bold blue]\n"
        "[dim]一个支持MCP协议的AI助手客户端[/dim]",
        border_style="blue"
    ))
    
    # 检查环境变量
    if not os.getenv("LLM_API_KEY"):
        console.print("⚠️  提示: 未设置LLM_API_KEY环境变量，AI对话功能将受限", style="yellow")
        console.print("💡 请在.env文件中设置LLM相关配置以启用完整功能\n", style="dim")
    
    # 运行CLI
    cli()

if __name__ == "__main__":
    if sys.version_info >= (3, 7):
        # Python 3.7+ 使用 asyncio.run
        import asyncio
        import inspect
        
        # 修复click异步支持
        original_cli = cli
        def sync_cli():
            ctx = click.get_current_context()
            if ctx.invoked_subcommand:
                cmd = ctx.invoked_subcommand
                # 如果是异步命令，运行它
                if hasattr(original_cli.commands[cmd], 'callback') and inspect.iscoroutinefunction(original_cli.commands[cmd].callback):
                    return asyncio.run(original_cli.commands[cmd].callback(**ctx.params))
            return original_cli()
        
        # 为异步命令创建包装器
        for cmd_name, cmd in original_cli.commands.items():
            if hasattr(cmd, 'callback') and inspect.iscoroutinefunction(cmd.callback):
                original_callback = cmd.callback
                def make_sync_wrapper(async_func):
                    def sync_wrapper(*args, **kwargs):
                        return asyncio.run(async_func(*args, **kwargs))
                    return sync_wrapper
                cmd.callback = make_sync_wrapper(original_callback)
        
        cli()
    else:
        console.print("❌ 需要Python 3.7+版本", style="red")
        sys.exit(1) 