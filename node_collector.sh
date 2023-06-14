#!/bin/bash
base_url="http://127.0.0.1:3000"
#基本信息
time_day=$(date +%Y-%m-%d_%H:%M:%S)
time_cront=$(date +%Y%m%d%H%M%S)
time_cront_day=$(date +%Y%m%d)
time_file=$(date +%Y-%m-%d)
system_hostname=$(hostname | awk '{print $1}')

#获取服务器IP
system_ip=$(hostname -I | awk '{print $1}')

#获取总内存
mem_total=$(free -m | grep Mem | awk -F " " '{print $2}')

#获取剩余内存
mem_free=$(free -m | grep "Mem" | awk '{print $4}')

#获取已用内存
mem_use=$(free -m | grep Mem | awk -F " " '{print $2-$4-$6}')

#系统进程数
load_1=$(ps -ef | wc -l)

#僵尸进程数
load_5=$(top -b -n 1 | grep Tasks | awk '{print $10}')

#CPU空闲id
load_15=$(top -b -n 1 | grep Cpu | awk -F',' '{print $4}' | awk -F'id' '{print $1}')

#过滤磁盘使用率大于60%目录，并加入描述
#disk_1=$(df -Ph | awk '{if(+$5>15) print "分区:"$6,"总空间:"$2,"使用空间:"$3,"剩余空间:"$4,"磁盘使用率:"$5}')
disk_most=$(df -P | awk '{if(+$5>0) print $2,$6,$1}' | grep -v ":/" | sort -nr | awk '{print $2}' | head -1)
disk_f=$(df -Ph | grep "$disk_most"$ | awk '{if(+$5>0) print $6}')
disk_total=$(df -Ph | grep "$disk_most"$ | awk '{if(+$5>0) print $2}')
disk_free=$(df -Ph | grep "$disk_most"$ | awk '{if(+$5>0) print $4}')
disk_per=$(df -Ph | grep "$disk_most"$ | awk '{if(+$5>0) print $5}')
disk_per_1=$(echo $disk_per | sed 's/%//g')

disk_ux=$(df -P | awk '{if(+$5>0) print $5,$1}' | grep -v ":/" | sort -nr | awk -F"%" '{print $1}' | head -1)
if [[ $disk_ux -gt 60 ]]; then
	#分区
	disk_f_60=$(df -P | awk '{if(+$5>0) print $5,$1,$6}' | grep -v ":/" | sort -nr | awk '{print $3}' | head -1)
	#磁盘使用率
	disk_per_60=$(df -P | awk '{if(+$5>0) print $5,$1,$6}' | grep -v ":/" | sort -nr | awk '{print $1}' | head -1)
	disk_status=不正常
else
	#分区
	disk_f_60=无
	#磁盘使用率
	disk_per_60=无
	disk_status=正常
fi
#文件路径
CHECK_HOME="$(
	cd "$(dirname "$0")"
	pwd
)"
path="$CHECK_HOME"/monitor_system_check_"$time_file".txt

#内存阈值
mem_mo='70'
PERCENT=$(printf "%d%%" $(($mem_use * 100 / $mem_total)))
PERCENT_1=$(echo $PERCENT | sed 's/%//g')
if [[ $PERCENT_1 -gt $mem_mo ]]; then
	mem_status_total="$mem_total"MB
	mem_status_use="$mem_use"MB
	mem_status_per=$PERCENT
	mem_status=不正常
else
	mem_status_total="$mem_total"MB
	mem_status_use=$"$mem_use"MB
	mem_status_per=$PERCENT
	mem_status=正常
fi

is_init=false
if [[ ! -s $path ]] || [[ $(grep 统计时间 $path | wc -l) -eq 0 ]]; then
	echo -e "主机名 统计时间 服务器IP 系统进程数 僵尸进程数 CPU空闲id 总内存大小 已用内存 内存使用率 内存巡检状态 分区 数据盘总空间 数据盘剩余空间 数据盘磁盘使用率 磁盘超60使用分区 磁盘超60使用率 磁盘状况" >>$path
	is_init=true
fi
# echo -e "input: $time_day $system_hostname $system_ip $load_1 $load_15 $load_5 $mem_status_total $mem_status_use $mem_status_per $mem_status $disk_f $disk_total $disk_free $disk_per $disk_f_60 $disk_per_60 $disk_status" >>$path
# 将变量以json形式打印
node_json="{\"system_hostname\":\"$system_hostname\",\"time_day\": \"$time_day\", \"system_ip\": \"$system_ip\", \"load_1\": $load_1, \"load_5\": $load_5, \"load_15\": $load_15, \"mem_status_total\": \"$mem_status_total\", \"mem_status_use\": \"$mem_status_use\", \"mem_status_per\": $PERCENT_1, \"mem_status\": \"$mem_status\", \"disk_f\": \"$disk_f\", \"disk_total\": \"$disk_total\", \"disk_free\": \"$disk_free\", \"disk_per\": $disk_per_1, \"disk_f_60\": \"$disk_f_60\", \"disk_per_60\": \"$disk_per_60\", \"disk_status\": \"$disk_status\"}"
echo -e "input: $node_json" >>$path
# 定义变量
# 发送HTTP POST请求
# if [[ $is_init == true ]]; then
	# echo "初始化 首次上报" >>$path
	# curl_output=$(curl --trace-ascii $path -d "$node_json" -H "Content-Type: application/json" -X POST "${base_url}/nodes")
	curl_output=$(curl -d "$node_json" -H "Content-Type: application/json" -X POST "${base_url}/nodes")
# else
	# echo "已初始化 开始定期上报" >>$path
	# curl_output=$(curl --trace-ascii $path -d "$node_json" -H "Content-Type: application/json" -X POST "${base_url}/nodes/${system_hostname}")
	# curl_output=$(curl -d "$node_json" -H "Content-Type: application/json" -X POST "${base_url}/nodes/${system_hostname}")
# fi
echo "output: $curl_output" >>$path