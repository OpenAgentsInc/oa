# Project File Hierarchy

```
├── 
  ├── .dockerignore
  ├── .sqlx
    ├── query-07998c130b6010768555a55965ffe480da2e1961e29c4598ce2b55a147c3f2a0.json
    ├── query-793f0df728d217c204123f12e4eafd6439db2d49d0cb506618ae9e780c7e0558.json
    ├── query-ed279fc2dda0c3ede3e81a4500fcaa9da2220f8a9ad6c1debc3095deb9f84759.json
  ├── Cargo.toml
  ├── Dockerfile
  ├── README.md
  ├── configuration
    ├── base.yaml
    ├── local.yaml
    ├── production.yaml
  ├── docs
    ├── chat-sharing.md
    ├── githooks.md
    ├── sync-engine-roadmap.md
    ├── sync-engine.md
  ├── migrations
    ├── 20200823135036_create_subscriptions_table.sql
    ├── 20240101000000_create_shared_conversations_table.sql
    ├── 20240101000001_enable_uuid.sql
  ├── public
    ├── favicon.ico
    ├── index.html
    ├── js
      ├── LightingSystem.js
      ├── OnyxOrb.js
      ├── SceneSystem.js
      ├── ViewSystem.js
      ├── main.js
    ├── logo.png
    ├── principles.css
    ├── principles.html
    ├── style.css
  ├── scripts
    ├── generate_hierarchy.rs
    ├── init_db.sh
  ├── spec.yaml
  ├── src
    ├── configuration.rs
    ├── lib.rs
    ├── main.rs
    ├── routes
      ├── chats.rs
      ├── health_check.rs
      ├── mod.rs
      ├── subscriptions.rs
    ├── startup.rs
    ├── telemetry.rs
  ├── tests
    ├── health_check.rs
```
