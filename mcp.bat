@echo off
REM 🤖 Grape MCP DevTools 便捷启动脚本

if "%1"=="" (
    powershell.exe -ExecutionPolicy Bypass -File "start_mcp_server.ps1" start
) else (
    powershell.exe -ExecutionPolicy Bypass -File "start_mcp_server.ps1" %1
) 