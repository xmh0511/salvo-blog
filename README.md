# Salvo Blog

基于 **Rust + Salvo + SeaORM + Tera** 的全栈博客系统，支持公开阅读、分级权限、内容创作、评论互动与站内消息提醒。

## 功能亮点

### 1. 内容浏览与发现
- 首页文章流（分页）
- 热文排行
- 文章详情页
- 阅读量统计
- 全站搜索（标题 / 正文 / 标签）

### 2. 用户认证与账号体系
- 用户注册（邮箱验证码）
- 用户登录（支持“记住我”）
- 忘记密码 / 密码重置（邮箱验证码）
- JWT 鉴权（Cookie 携带 token）

### 3. 文章创作与管理
- 发布文章
- 编辑文章
- 文章软删除（显示/隐藏切换）
- 个人文章列表管理（分页）
- 标签分类
- 阅读权限分级（含“仅自己可见”）

### 4. Markdown 富文本能力
- Editor.md 编辑器
- Markdown 实时预览
- 代码高亮
- Emoji
- 任务列表
- LaTeX 公式
- 流程图 / 时序图
- 图片上传（`/upload`）

### 5. 评论与互动
- 文章评论发布
- 评论编辑 / 删除（仅本人）
- 评论锚点跳转（定位到具体楼层）
- `@提及` 触发消息通知

### 6. 站内信系统
- 未读消息统计
- 最近消息摘要
- 消息列表页
- 消息已读标记
- 前端轮询刷新消息（10 秒）

### 7. 个人资料
- 个人信息页
- 头像更新

### 8. 工程与性能实践
- SeaORM + 原生 SQL 混合查询
- 列表页批量补全元数据（避免 N+1）
- 全局统一错误响应（HTML / JSON）
- 静态资源压缩（gzip）
- 生产环境滚动日志

## 技术栈
- **后端**：Rust, Salvo
- **ORM**：SeaORM
- **模板**：Tera
- **数据库**：MySQL
- **缓存**：Redis
- **邮件**：Resend
- **前端**：Layui, jQuery, Editor.md

## 项目结构

```text
salvo-blog
├── src/
│   ├── main.rs          # 应用入口、路由注册
│   └── home.rs          # 主要业务处理（鉴权/文章/评论/消息/搜索）
├── views/               # Tera 页面模板
├── public/              # 静态资源（CSS/JS/Markdown 组件）
├── deno.sql             # MySQL 表结构脚本
├── config.json          # 运行配置
└── README.md
```

## 快速开始

### 1) 准备依赖
- Rust（建议 stable 最新版）
- MySQL
- Redis

### 2) 初始化数据库
执行项目根目录下的 `deno.sql` 导入表结构（该文件名为历史命名的 SQL 初始化脚本）。

示例命令：`mysql -u <user> -p -D <database> < deno.sql`

### 3) 配置 `config.json`
在 `./config.json` 中配置：

```json
{
  "base_url": "/",
  "database_url": "mysql://<user>:<password>@<host>/<database>",
  "bind_addr": "0.0.0.0:8080",
  "secret_key": "<jwt_secret>",
  "redis_url": "redis://127.0.0.1/",
  "resend_key": "<resend_api_key>"
}
```

### 4) 启动项目

```bash
cd salvo-blog
cargo run
```

默认访问：`http://127.0.0.1:8080`

## 页面预览

![](https://github.com/xmh0511/salvo-blog/blob/main/preview/preview0.png)
![](https://github.com/xmh0511/salvo-blog/blob/main/preview/preview1.png)
![](https://github.com/xmh0511/salvo-blog/blob/main/preview/preview2.png)
![](https://github.com/xmh0511/salvo-blog/blob/main/preview/preview3.png)
![](https://github.com/xmh0511/salvo-blog/blob/main/preview/preview4.png)
![](https://github.com/xmh0511/salvo-blog/blob/main/preview/preview5.png)
![](https://github.com/xmh0511/salvo-blog/blob/main/preview/preview6.png)
![](https://github.com/xmh0511/salvo-blog/blob/main/preview/preview7.png)
![](https://github.com/xmh0511/salvo-blog/blob/main/preview/preview8.png)
![](https://github.com/xmh0511/salvo-blog/blob/main/preview/preview9.png)
![](https://github.com/xmh0511/salvo-blog/blob/main/preview/preview10.png)
![](https://github.com/xmh0511/salvo-blog/blob/main/preview/preview11.png)
