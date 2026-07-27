# Pioneer

Pioneer 是一个基于 Bevy 引擎开发的游戏项目。它是一个星球探索与模拟建造游戏。旨在成为一款兼具内容与深度，同时具有高度易玩性的游戏。

灵感来源：

- Stationeer
- The Planet Crafter

## 当前技术基线

- Rust 1.95.0+
- Bevy: `0.19.0`
- 序列化: `serde`
- 日志: `tracing`

## 项目约定 Convention

- [基础规范](docs/project-conventions.md)

## 目标 Target

### 基建 Foundation

基建描述一切基于 Bevy 引擎的基础设施。具备一定通用性，但根本上服务于 Pioneer。

#### 资产管理 AssetManager

资产管理的存在亟待解决以下问题：

- 对资产加载进行管理。
    - [x] [High]屏蔽 git-lfs 指针文件受加载。
    - [ ] [Low]默认资产。

### 游戏玩法 Gameplay
