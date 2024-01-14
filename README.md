# cases

中国裁判文书网搜索

## 用法

> [!CAUTION]
> 需要磁盘空间 320G 以上，可能需要数小时的时间


### 下载原始数据（102G）

1. 通过bt下载，种子文件为 `810air.torrent` ，可以从本仓库下载，也可以通过：https://files.catbox.moe/810air.torrent
2. https://pan.baidu.com/s/15jiY3DEpED7ywl-gfPFkYw?pwd=4QrB 提取码：4QrB 

原始数据来源于[马克数据网](https://www.macrodatas.cn/article/1147471898)，文书数量超过8500万，约102G。下载后**不要**解压子文件，将文件路径填写到 `config.toml` 中的 `raw_data_path` 变量中；

### 将数据加载到 rocksdb 数据库中

运行 `convert` 程序。此过程会将原始数据放入 rocksdb 数据库中，数据库文件路径为 `config.toml` 中的 `db` 变量；转换后的数据大小约为 200G，转换可能会花费数小时的时间；如果中途中断，再次运行会从中断处继续。

### 创建索引
运行 `index` 程序会将数据库中的数据创建索引，索引文件路径为 `config.toml` 中的 `index_path` 变量；如果中途中断，需要删除 `index_path` 中的文件，重新运行 `index` 程序；默认情况下，不会索引案件内容，如果需要索引案件内容，需要将 index.rs 文件中对应的注释去掉（相应索引文件会超过150G）。索引大小约为 15.5G，转换可能会花费数小时的时间；

### 运行搜索服务
运行 `main` 程序，用浏览器打开网址，即可搜索。