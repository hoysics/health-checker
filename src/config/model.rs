use schemars::schema::RootSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tracing::info;

#[derive(Serialize, Deserialize, Debug)]
pub struct Profiles {
    pub active: String,
}
// 用来接收application.yml解析结果
#[derive(Serialize, Deserialize, Debug)]
pub struct EnvConfig {
    pub profiles: Profiles,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Bootstrap {
    pub server: Server,
    pub services: Vec<Service>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Server {
    pub addr: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Service {
    pub name: String,
    pub api: String,
}

// 加载指定配置文件
fn load_config<T>(path: &str) -> Option<T>
where
    T: DeserializeOwned,
{
    // 1.通过std::fs读取配置文件内容
    // 2.通过serde_yaml解析读取到的yaml配置转换成json对象
    match serde_yaml::from_str::<RootSchema>(
        &std::fs::read_to_string(path).expect(&format!("failure read file {}", path)),
    ) {
        Ok(root_schema) => {
            // 通过serde_json把json对象转换指定的model
            let data =
                serde_json::to_string_pretty(&root_schema).expect("failure to parse RootSchema");
            let config = serde_json::from_str::<T>(&*data)
                .expect(&format!("failure to format json str {}", &data));
            // 返回格式化结果
            Some(config)
        }
        Err(err) => {
            // 记录日志
            info!("{}", err);
            // 返回None
            None
        }
    }
}

// 加载目标文件application.yml
fn load_env_config() -> Option<EnvConfig> {
    load_config::<EnvConfig>("application.yml")
}
// 根据环境加载application-{}.yml文件
fn load_bootstrap_config_from_env(active: String) -> Option<Bootstrap> {
    let path = format!("application-{}.yml", active);
    load_config::<Bootstrap>(&path)
}
// 真正对外暴露的方法，根据application.yml指定的环境变量动态加载对应的配置文件
pub fn load_bootstrap_config() -> Option<Bootstrap> {
    if let Some(env_config) = load_env_config() {
        return load_bootstrap_config_from_env(env_config.profiles.active);
    }
    None
}

#[cfg(test)]
mod test {
    use crate::config::model::*;

    #[test]
    pub fn load_config_test() {
        match load_bootstrap_config() {
            None => {
                println!("None");
            }
            Some(config) => {
                println!("{:#?}", config);
            }
        }
    }
}
