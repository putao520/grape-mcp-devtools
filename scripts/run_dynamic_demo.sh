#!/bin/bash

# 动态MCP工具注册系统演示脚本

echo "🚀 动态MCP工具注册系统演示"
echo "================================================================"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 函数：打印带颜色的标题
print_title() {
    echo -e "\n${BLUE}🎯 $1${NC}"
    echo -e "${BLUE}$(printf '=%.0s' {1..50})${NC}"
}

# 函数：打印步骤
print_step() {
    echo -e "\n${GREEN}▶ $1${NC}"
}

# 检查是否在项目根目录
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}❌ 错误: 请在项目根目录运行此脚本${NC}"
    exit 1
fi

print_title "构建项目"
print_step "编译动态MCP服务器..."
cargo build --bin dynamic-mcp-server
if [ $? -ne 0 ]; then
    echo -e "${RED}❌ 构建失败${NC}"
    exit 1
fi

print_title "CLI工具检测演示"
print_step "检测当前环境中的CLI工具..."
cargo run --bin dynamic-mcp-server detect --verbose

print_title "策略信息展示"
print_step "显示所有可用的注册策略..."
cargo run --bin dynamic-mcp-server strategies

print_title "默认策略演示 (OnlyAvailable)"
print_step "使用默认策略检测并准备注册工具..."
cargo run --bin dynamic-mcp-server -- --report-only

print_title "强制注册策略演示 (ForceAll)"
print_step "强制注册所有工具..."
cargo run --bin dynamic-mcp-server -- --all --report-only

print_title "基于特性的策略演示 (FeatureBased)"
print_step "仅注册构建工具和包管理器..."
cargo run --bin dynamic-mcp-server -- --feature build-tool --feature package-manager --report-only

print_title "完整演示示例"
print_step "运行完整的演示程序..."
cargo run --example dynamic_registry_demo

print_title "测试执行"
print_step "运行CLI检测相关测试..."
cargo test cli_detection_tests --verbose

print_title "性能测试"
print_step "测试大规模工具检测性能..."
time cargo run --bin dynamic-mcp-server detect

echo ""
print_title "演示完成"
echo -e "${GREEN}✅ 动态MCP工具注册系统演示已完成！${NC}"
echo ""
echo -e "${YELLOW}💡 提示:${NC}"
echo "• 使用 'cargo run --bin dynamic-mcp-server --help' 查看所有选项"
echo "• 使用 'cargo run --bin dynamic-mcp-server serve' 启动实际的MCP服务器"
echo "• 使用 'cargo test' 运行所有测试"
echo ""
echo -e "${BLUE}🎯 下一步:${NC}"
echo "1. 根据你的环境调整工具检测列表"
echo "2. 添加更多自定义的MCP工具映射"
echo "3. 配置生产环境的注册策略"
echo "" 