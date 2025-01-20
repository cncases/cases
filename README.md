# cases

中国裁判文书网搜索

## 用法

> [!CAUTION]
> 需要磁盘空间 320G 以上，可能需要数小时的时间


### 0. 下载程序并创建配置文件

方法一：从 releases 页面下载已编译好的二进制文件（推荐），https://github.com/cncases/cases/releases

方法二：自行编译

```bash
## 安装 rust
https://www.rust-lang.org/tools/install

## clone 本仓库
git clone https://github.com/cncases/cases.git

## 编译，对应程序在 target/release/ 文件夹中
cargo build -r 
```

配置文件参考[config.toml](./config.toml)

### 1. 下载原始数据（102G）

方法：通过bt下载，种子文件为 `810air.torrent` ，可以从本[仓库](./810air.torrent)下载，也可以通过链接 https://files.catbox.moe/810air.torrent

原始数据来源于[马克数据网](https://www.macrodatas.cn/article/1147471898)，文书数量超过8500万，约102G。下载后**不要**解压子文件，将文件路径填写到 `config.toml` 中的 `raw_data_path` 变量中；

### 2. 将数据加载到数据库中

运行 `convert config.toml` 程序。此过程会将原始数据放入数据库中，数据库文件路径为 `config.toml` 中的 `db` 变量；转换后的数据大小约为 200G，转换可能会花费数小时的时间；如果中途中断，再次运行会从中断处继续。

### 3. 创建索引
运行 `index config.toml` 程序会将数据库中的数据创建索引，索引文件路径为 `config.toml` 中的 `index_path` 变量；如果中途中断，需要删除 `index_path` 中的文件，重新运行 `index` 程序；默认情况下，不会索引案件内容，索引大小约为 15.5G，可能会花费数小时的时间。如果需要索引案件内容，需要将index.toml中的 `index_with_full_text` 设置为 `true`，但是这会使索引文件增加到150G左右，索引时间也会增加到十几个小时。

### 4. 运行搜索服务
运行 `main config.toml` 程序，用浏览器打开`config.toml`网址，即可搜索。

## 说明

当程序和配置文件放在同一目录下，且配置文件命名为 `config.toml` 时，可以省略配置文件路径参数。

![screenshot](Screenshot.png)
