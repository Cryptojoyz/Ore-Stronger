# Ore 修改版 2.3.0

## 功能

已同步官方最新版本，如果觉得好用，请给我点点关注：[Twitter](https://x.com/YTDiscovery921)

### 同步更新了官方最新版本，目前官方版本自带功能:
* 寻找最好的bus提交
* 支持主流RPC节点服务商的动态Gas
* 显示挖矿过程中已经挖到的最好难度
* 提交成功显示时间戳
* 支持Jito提交
* 从Ox0错误中恢复

### 增加的功能:
* 发送阈值：18   难度大于18的Hash会直接进行发送。
* 再战一轮： 如果在进行挖矿后难度小于18, 在70s内会重新运算Hash, 如果找到难度大于18的Hash就直接发送，如果没找到，则在时间到70s后发送这期间的最优Hash。
* 动态Gas： **如果不使用Jito，使用动态Gas的话**，在官方动态Gas的基础上加了判断，如果挖到难度大于等于27的Hash，会在推荐Gas上*2作为小费，最大限度确保上链。
* 优化再战一轮的Hash计算方式，最大可能找到更好的Hash。 

我自己用的付费节点推荐：[Quiknode](https://www.quicknode.com/?via=yt)

## 使用

首先，
```sh
git clone 项目地址
```

然后进入到项目，运行
```sh
cargo build --release
```

完成后运行
```sh
cd target/release
```

最后和官方的用法一样
```sh
./ore mine 
```
## 使用注意
**Dynamic Fee功能和Jito不要叠加使用，会重复给小费**，使用Jito时，一定要把 Priority Fee 设置为0， 否则默认值是500000，建议使用命令为
```sh
ore mine --rpc 你的节点 --cores 你的核心数 --keypair 你的密钥路径 --priority-fee 0 --jito
```

使用Quicknode节点动态费用的，需要在节点的Add-ons中的Solana Priority Fee API

预设的重置间隔为5000，这是基于我自己电脑算力的值，大家在使用的时候需要根据自己电脑的运算时间，让重置间隔的运算时间大约在20s左右，这样可以刚好在70s左右发出Hash上链，否则时间太久会导致奖励继续减半，得不偿失。
修改位置在 mine.rs 文件 interval = 5000 这里，直接改后面的数字即可。

修改完成后需要重新运行一下
```sh
cargo build --release
```
才会生效

## Ore动态惩罚机制介绍

Ore采用迟到惩罚机制，每超时一分钟，奖励**减半**，不足一分钟的超时秒数按照如下公式计算奖励削减。

![惩罚](./formula/QianJianTec1723374388895.jpg)

![最后奖励](./formula/QianJianTec1723374376193.jpg)



