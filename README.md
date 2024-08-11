# Ore 修改版 2.2.1

## 功能

### 同步更新了官方最新版本，目前官方版本自带功能:
* 寻找最好的bus提交
* 支持主流RPC节点服务商的动态Gas
* 显示挖矿过程中已经挖到的最好难度
* 提交成功显示时间戳

### 增加的功能:
* 发送阈值：18   难度大于18的Hash会直接进行发送。
* 再战一轮： 如果在进行挖矿后难度小于18, 在70s内会重新运算Hash, 如果找到难度大于18的Hash就直接发送，如果没找到，则在时间到70s后发送这期间的最优Hash。
* 动态Gas： 在官方动态Gas的基础上加了判断，如果挖到难度大于等于27的Hash，会在推荐Gas上*2作为小费，最大限度确保上链。 目前动态Gas的策略仅支持Helius、Triton、Alchemy、[Quiknode(我自己用的，价格比较便宜)](https://www.quicknode.com/?via=yt)

如果觉得好用，请给我点点关注：[Twitter](https://x.com/YTDiscovery921)

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
使用Quicknode节点的，需要在节点的Add-ons中的Solana Priority Fee API

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



