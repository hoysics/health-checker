

// 记录数据同时判断是否需要报警
pub struct Monitor {
    db: HashMap<String, Node>,//存储原始的节点信息
    dc: Doctor,
}

impl Monitor {
    fn new(dc:Doctor) ->Monitor{
        Monitor {db:HashMap::new(), dc}
    }
    fn log(&self,event: Event){
        //TODO: 带颜色的打印控制台日志
        //1. 打印日志 记录节点状态
        //2. 将节点状况计入Map中
        //3. 根据不同情况 决定是否立即邮件抱紧
        match event {
            Event::Heartbeat(info) => {
                match info.target {
                    Target::Node(id, node) => self.update_node(id,node),
                    Target::Service(name, service) => self.update_service(name,service),
                };
            },
            Event::Offline(target) => self.offline(target),
        };
    }
    fn update_node(&self,id: String,node:Option<Node>) {
        
    }
    fn update_service(&self,name:String,service:Option<Service>) {
        
    }
    fn offline(&self,target:Target) {
        
    }
    fn tranverse_check(&self){
        let mut result: Vec<HealthInfo>=Vec::new();
        //TODO: 遍历Map 检查每个节点的健康状态
        for (id,node) in self.db.iter()   {
            let (status,msg)=self.dc.check_node(node);
            let mut n=node.clone();
            n.status_msg=Some(msg);
            result.push(HealthInfo { target: Target::Node(String::from(id), Some(n)), status });
        };
        //1. 将状态先输出至单独的本地文件 用以留档
        println!("finished check nodes\n{:?}",result);
        //2. 发邮件通知当前的文件健康状态
    }
    // fn trace(&self,node: &Node){
    //
    // }
    // fn warn(&self,node: &Node){
    //
    // }
    // fn error(&self,node: &Node){
    //
    // }
    // fn offline(&self,node: String){
    //
    // }
}
