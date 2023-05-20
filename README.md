# Health Checker

学习性质的服务监控工具

由Rust语言开发，用于监控非云原生情况下、传统虚拟机形式的小型服务集群的监控工具，特别针对没有资源额外支持Prometheus等中间件部署的场景。

本工具RoadMap见下：

1. 通过脚本+Http服务器的形式收集服务器资源+服务响应状况的数据，超出限额时，通过邮件通知提醒。基于Axum实现，采用本地配置文件加载所有配置

