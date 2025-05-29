#!/usr/bin/env pwsh
<#
.SYNOPSIS
    🚀 Grape MCP DevTools 服务器管理脚本
.DESCRIPTION
    用于启动、停止和管理MCP服务器进程
.PARAMETER Action
    操作类型: start, stop, status, restart
.EXAMPLE
    .\start_mcp_server.ps1 start
    .\start_mcp_server.ps1 stop
    .\start_mcp_server.ps1 status
#>

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet("start", "stop", "status", "restart", "logs")]
    [string]$Action = "start"
)

$ServerName = "grape-mcp-devtools"
$LogFile = "mcp_server.log"
$PidFile = "mcp_server.pid"

function Write-ColoredOutput {
    param(
        [string]$Message,
        [string]$Color = "White"
    )
    Write-Host $Message -ForegroundColor $Color
}

function Get-MCPServerProcess {
    """获取MCP服务器进程"""
    return Get-Process -Name "grape-mcp-devtools" -ErrorAction SilentlyContinue
}

function Start-MCPServer {
    """启动MCP服务器"""
    Write-ColoredOutput "🚀 启动MCP服务器..." "Blue"
    
    # 检查是否已经运行
    $existingProcess = Get-MCPServerProcess
    if ($existingProcess) {
        Write-ColoredOutput "⚠️  MCP服务器已经在运行 (PID: $($existingProcess.Id))" "Yellow"
        return
    }
    
    # 检查Rust项目
    if (!(Test-Path "Cargo.toml")) {
        Write-ColoredOutput "❌ 未找到Cargo.toml，请在项目根目录运行此脚本" "Red"
        return
    }
    
    try {
        # 启动服务器进程
        $process = Start-Process -FilePath "cargo" -ArgumentList "run", "--bin", $ServerName -NoNewWindow -PassThru -RedirectStandardOutput $LogFile -RedirectStandardError "mcp_server_error.log"
        
        # 等待进程启动
        Start-Sleep -Seconds 3
        
        # 检查进程是否还在运行
        if (!$process.HasExited) {
            # 保存PID
            $process.Id | Out-File -FilePath $PidFile -Encoding utf8
            Write-ColoredOutput "✅ MCP服务器启动成功! PID: $($process.Id)" "Green"
            Write-ColoredOutput "📋 日志文件: $LogFile" "Cyan"
            Write-ColoredOutput "🔍 使用 '.\start_mcp_server.ps1 status' 检查状态" "Cyan"
            Write-ColoredOutput "🛑 使用 '.\start_mcp_server.ps1 stop' 停止服务器" "Cyan"
        } else {
            Write-ColoredOutput "❌ MCP服务器启动失败" "Red"
            Write-ColoredOutput "📋 查看错误日志: mcp_server_error.log" "Yellow"
        }
    }
    catch {
        Write-ColoredOutput "❌ 启动失败: $_" "Red"
    }
}

function Stop-MCPServer {
    """停止MCP服务器"""
    Write-ColoredOutput "🛑 停止MCP服务器..." "Yellow"
    
    # 从PID文件读取
    if (Test-Path $PidFile) {
        $pid = Get-Content $PidFile -Raw
        $pid = $pid.Trim()
        
        try {
            $process = Get-Process -Id $pid -ErrorAction Stop
            Stop-Process -Id $pid -Force
            Write-ColoredOutput "✅ MCP服务器已停止 (PID: $pid)" "Green"
            Remove-Item $PidFile -ErrorAction SilentlyContinue
        }
        catch {
            Write-ColoredOutput "⚠️  进程 $pid 不存在或已停止" "Yellow"
            Remove-Item $PidFile -ErrorAction SilentlyContinue
        }
    }
    
    # 备用方案：查找并停止所有相关进程
    $processes = Get-MCPServerProcess
    if ($processes) {
        foreach ($proc in $processes) {
            Stop-Process -Id $proc.Id -Force
            Write-ColoredOutput "✅ 停止进程: $($proc.Id)" "Green"
        }
    } else {
        Write-ColoredOutput "ℹ️  没有找到运行中的MCP服务器进程" "Cyan"
    }
}

function Get-MCPServerStatus {
    """获取MCP服务器状态"""
    Write-ColoredOutput "🔍 检查MCP服务器状态..." "Blue"
    
    $processes = Get-MCPServerProcess
    if ($processes) {
        Write-ColoredOutput "✅ MCP服务器正在运行:" "Green"
        foreach ($proc in $processes) {
            $startTime = $proc.StartTime.ToString("yyyy-MM-dd HH:mm:ss")
            $cpuTime = [math]::Round($proc.CPU, 2)
            $memory = [math]::Round($proc.WorkingSet / 1MB, 2)
            
            Write-ColoredOutput "   📋 PID: $($proc.Id)" "Cyan"
            Write-ColoredOutput "   ⏰ 启动时间: $startTime" "Cyan"
            Write-ColoredOutput "   💾 内存: ${memory}MB" "Cyan"
            Write-ColoredOutput "   ⚡ CPU时间: ${cpuTime}s" "Cyan"
        }
        
        # 检查日志文件
        if (Test-Path $LogFile) {
            $logSize = [math]::Round((Get-Item $LogFile).Length / 1KB, 2)
            Write-ColoredOutput "   📄 日志大小: ${logSize}KB" "Cyan"
        }
    } else {
        Write-ColoredOutput "❌ MCP服务器未运行" "Red"
        
        # 检查是否有僵尸PID文件
        if (Test-Path $PidFile) {
            Write-ColoredOutput "⚠️  发现孤立的PID文件，正在清理..." "Yellow"
            Remove-Item $PidFile
        }
    }
}

function Show-MCPServerLogs {
    """显示MCP服务器日志"""
    Write-ColoredOutput "📋 MCP服务器日志 (最近20行):" "Blue"
    
    if (Test-Path $LogFile) {
        Get-Content $LogFile -Tail 20 | ForEach-Object {
            Write-ColoredOutput $_ "White"
        }
        Write-ColoredOutput "`n💡 完整日志文件: $LogFile" "Cyan"
    } else {
        Write-ColoredOutput "❌ 日志文件不存在: $LogFile" "Red"
    }
    
    if (Test-Path "mcp_server_error.log") {
        Write-ColoredOutput "`n❌ 错误日志:" "Red"
        Get-Content "mcp_server_error.log" -Tail 10 | ForEach-Object {
            Write-ColoredOutput $_ "Red"
        }
    }
}

function Restart-MCPServer {
    """重启MCP服务器"""
    Write-ColoredOutput "🔄 重启MCP服务器..." "Blue"
    Stop-MCPServer
    Start-Sleep -Seconds 2
    Start-MCPServer
}

function Show-Usage {
    """显示使用说明"""
    Write-ColoredOutput "🤖 Grape MCP DevTools 服务器管理脚本" "Blue"
    Write-ColoredOutput "=" * 50 "Blue"
    Write-ColoredOutput ""
    Write-ColoredOutput "用法:" "White"
    Write-ColoredOutput "  .\start_mcp_server.ps1 [action]" "Cyan"
    Write-ColoredOutput ""
    Write-ColoredOutput "可用操作:" "White"
    Write-ColoredOutput "  start   - 启动MCP服务器 (默认)" "Green"
    Write-ColoredOutput "  stop    - 停止MCP服务器" "Red"
    Write-ColoredOutput "  status  - 检查服务器状态" "Yellow"
    Write-ColoredOutput "  restart - 重启服务器" "Blue"
    Write-ColoredOutput "  logs    - 显示服务器日志" "Cyan"
    Write-ColoredOutput ""
    Write-ColoredOutput "示例:" "White"
    Write-ColoredOutput "  .\start_mcp_server.ps1 start" "Cyan"
    Write-ColoredOutput "  .\start_mcp_server.ps1 status" "Cyan"
    Write-ColoredOutput "  .\start_mcp_server.ps1 stop" "Cyan"
}

# 主逻辑
switch ($Action.ToLower()) {
    "start" { Start-MCPServer }
    "stop" { Stop-MCPServer }
    "status" { Get-MCPServerStatus }
    "restart" { Restart-MCPServer }
    "logs" { Show-MCPServerLogs }
    default { Show-Usage }
} 