# 使用多阶段构建来优化镜像大小

# 构建阶段
FROM rust:1.77-slim-bookworm as builder

WORKDIR /usr/src/app
COPY . .

# 安装构建依赖
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# 构建应用
RUN cargo build --release

# 运行阶段
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && \
    apt-get install -y ca-certificates libssl-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# 复制可执行文件和配置
COPY --from=builder /usr/src/app/target/release/grape-mcp-devtools /usr/local/bin/grape-mcp-devtools
COPY --from=builder /usr/src/app/config /etc/grape-mcp-devtools/config

# 创建非 root 用户
RUN groupadd -r mcp && useradd -r -g mcp mcp

# 创建必要的目录并设置权限
RUN mkdir -p /var/lib/mcp-data /var/cache/mcp && \
    chown -R mcp:mcp /var/lib/mcp-data /var/cache/mcp

# 切换到非 root 用户
USER mcp

# 设置环境变量
ENV MCP_CONFIG_PATH=/etc/grape-mcp-devtools/config/default.toml
ENV MCP_VECTOR_DB_STORAGE_PATH=/var/lib/mcp-data
ENV MCP_CACHE_DIR=/var/cache/mcp
ENV MCP_HOST=0.0.0.0
ENV MCP_PORT=8080

# 公开端口
EXPOSE 8080

# 设置数据卷
VOLUME ["/var/lib/mcp-data", "/var/cache/mcp"]

# 运行应用
CMD ["grape-mcp-devtools"]
