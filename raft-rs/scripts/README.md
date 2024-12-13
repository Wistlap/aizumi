# 測定に用いるスクリプト群

測定やグラフを自動で出してくれるスクリプト群

## スクリプト群

| スクリプト名 | 説明 |
|:-------------|:----------|
| autotest.sh | 1回テストを実行するスクリプト |
| autotest\_all.sh | 複数回テストを実行するスクリプト |
| log2csv.sh | mpstat のログを整形するスクリプト |
| plot\_from\_mpstat\_log.py  | log2csv.sh で整形したデータをグラフに出力するスクリプト |
| stat\_from\_csv.py| メッセージブローカが出力するログファイルから stat ファイルを出力するスクリプト |
| csv2stat\_all.sh | autotest\_all.sh が出力するログディレクトリ内の全てのログファイルに stat\_frpm\_csv.py を実行するスクリプト |
| stat2graph | stat\_from\_csv.py で出力した stat ファイルを含むログディレクトリからデータを整形しグラフを出力するスクリプト |
| stat2scrapbox | sta\t_from\_csv.py で出力した stat ファイルを含むログディレクトリからデータを整形し Scrapbox のテーブル記法で出力するスクリプト |
| raft_test.sh | Raft 実装のブローカに関する一連のテストとグラフ生成を実行するスクリプト|

## コマンド書式

### autotest.sh

```
./autotest.sh -b BROKER -r N -s N [-c N] [-d LEVEL] [-l LOG-FILE] [--replica N] [--runtime RUNTIME] [-- -BROKER-OPT...]
```
#### 説明

メッセージブローカ→レシーバ→センダの順に与えられた引数に則って起動を行いテストを行う．

#### オプション

***`-b BROKER`***

&nbsp; &nbsp; 使用するメッセージブローカのパスを指定する．

***`-c N`***

&nbsp; &nbsp; 送信するメッセージの回数を指定する．

***`-d LEVEL`***

&nbsp; &nbsp; 表示するメッセージのレベルを指定する．

***`-l LOG-FILE.log`***

&nbsp; &nbsp; メッセージブローカが出力するログファイルを指定する．また，mpstat コマンドを使用して CPU 使用率のグラフを出力する．

***`-r N`***

&nbsp; &nbsp; レシーバのプロセス数を指定する．

***`-s N`***

&nbsp; &nbsp; センダのプロセス数を指定する．

***`--replica N`***

&nbsp; &nbsp; レプリカのプロセス数を指定する．

***`--runtime RUNTIME`***

&nbsp; &nbsp; ブローカを動作させるランタイムを指定する． e.g. wasmer

***`-BROKER-OPT`***

&nbsp; &nbsp; メッセージブローカ毎に異なるオプションを指定する．e.g. -n

#### 実行可能なメッセージブローカの形式

autotest.sh を使用して実行するメッセージブローカが持つべきオプションは以下の通りである．

***`-b BROKER-PORT`***

&nbsp; &nbsp; メッセージブローカを実行するポート番号を指定する．

***`-d LEVEL`***

&nbsp; &nbsp; 表示するメッセージのレベルを指定する．

***`-p BROKER-PID-FILE`***

&nbsp; &nbsp; メッセージブローカの pid ファイルのパスを指定する．

***`-l LOG-FILE`***

&nbsp; &nbsp; メッセージブローカのログを出力するログファイルを指定する．

***`-BROKER-OPT`***

&nbsp; &nbsp; メッセージブローカ毎に異なるオプションを指定する．e.g. -n


#### 出力ファイル

-l オプションを使用することで以下のファイルが出力される．
```
LOG-FILE.log
LOG-FILE.config
LOG-FILE-mpstat.log
LOG-FILE-mpstat.tsv
LOG-FILE-mpstat.pdf
```

### autotest\_all.sh

```
./autotest_all.sh -b BROKER -r SEQ -s SEQ [-c N] [-d LEVEL] [-l LOG-FILE] [--replica SEQ] [--runtime RUNTIME] [-- -BROKER-OPT...]
```

#### 説明

指定した範囲でオプションの値を変更し，その値の組み合わせの回数 autotest.sh を実行する．

#### オプション

***`-b BROKER`***

&nbsp; &nbsp; 使用するメッセージブローカのパスを指定する．

***`-c N`***

&nbsp; &nbsp; 送信するメッセージの回数を指定する．

***`-d LEVEL`***

&nbsp; &nbsp; 表示するメッセージのレベルを指定する．

***`-l LOG-FILE`***

&nbsp; &nbsp; メッセージブローカが出力するログファイル名のフォーマットを指定し，この形式で autotest.sh 実行時にログファイルを出力する．

***`-r SEQ`***

&nbsp; &nbsp; テストを行うレシーバのプロセス数の範囲を指定する．

***`-s SEQ`***

&nbsp; &nbsp; テストを行うセンダのプロセス数の範囲を指定する．

***`--replica SEQ`***

&nbsp; &nbsp; テストを行うレプリカのプロセス数の範囲を指定する．

***`--runtime RUNTIME`***

&nbsp; &nbsp; ブローカを動作させるランタイムを指定する．e.g. wasmer

***`-BROKER-OPT`***

&nbsp; &nbsp; メッセージブローカ毎に異なるオプションを指定する．e.g. -n

#### -l オプション

-l オプションの引数 LOG-FILE に与えるべきフォーマットは以下の通りである．

&nbsp; &nbsp; `LOG-DIR/$(date +%Y%m%d-%H%M%S)/%b-%r-%s-%broker_option-%c-$(date +%Y%m%d-%H%M%S).log`

LOG\_DIR はログファイルを出力するディレクトリのパスである．autotest\_all.sh を実行している場所を始点とする相対パスを指定する．

ファイル名中の % で指定した部分は autotest.sh 実行時に与える同じ名前のオプションの引数の値を使用して置換される．

例として，%b は -b オプションの引数中のメッセージブローカ名で自動的に置換される．
ただし，%broker\_option はこの通りではなく -BROKER-OPT で指定したオプションと同じ文字を実行者が指定する必要がある．e.g. %n

### log2csv.sh

```
./log2csv.sh input_file
```

#### 説明

mpstat のログを整形して，input\_file.csv というファイルを生成する．autotest.sh の実行時に -l オプションで実行される．

#### 引数

***`input_file`***

&nbsp; &nbsp; mpstat で出力されたログファイル．

### plot\_from\_mpstat\_log.py

```
./plot_from_mpstat_log.py input_file output_file cores
```

#### 説明

log2csv.sh で出力されたファイルを入力としてグラフを出力する．出力するデータは，スクリプト内の ele\_name で指定する．autotest.sh の実行時に -l オプションで実行される．

#### 引数

***`input_file`***

&nbsp; &nbsp; log2csv.sh で出力されたファイル．

***`output_file`***

&nbsp; &nbsp; 出力されるファイル名．output\_file.pdf が出力される．

***`cores`***

&nbsp; &nbsp; 出力するグラフのコア番号を指定する．カンマ区切りでコアを列挙し，すべてのコアの平均値が見たい場合は，all を指定する．

### stat_from_csv.py

```
./stat_from_csv.py input_file
```

#### 説明

メッセージブローカが出力するログファイルからデータを整形して，input\_file.stat というファイルを生成する．

#### 引数

***`input_file`***

&nbsp; &nbsp; メッセージブローカが出力するログファイル．

### csv2stat_all.sh

```
./csv2stat_all.sh input_dir
```

#### 説明

autotest_all.sh が生成するログディレクトリ内の全てのログファイルに対して stat\_from\_csv.py を実行する．

#### 引数

***`input_dir`***

&nbsp; &nbsp; autotest\_all.sh によって出力されるログディレクトリ．

### stat2graph.py

```
./stat2graph.py input_dirs... (-n | -t) (-s | -b)
```

#### 説明

stat\_from\_csv.py が出力した stat ファイルを含むログディレクトリからデータを整形してスループットのグラフを出力する．

#### 引数

***`input_dirs...`***

&nbsp; &nbsp; stat ファイルを含むログディレクトリ．複数指定する場合はそれらのスループットの平均を取る．

#### オプション

***`-n`***

&nbsp; &nbsp; 実行したメッセージブローカがスレッドプール方式であることを指定する．このオプションを指定する場合，***`-t`*** オプションは指定できない．

***`-t`***

&nbsp; &nbsp; 実行したメッセージブローカがタイムアウト方式であることを指定する．このオプションを指定する場合，***`-n`*** オプションは指定できない．

***`-s`***

&nbsp; &nbsp; 出力するグラフの形式として散布図を指定する．このオプションを指定する場合，***`-b`*** オプションは指定できない．

***`-b`***

&nbsp; &nbsp; 出力するグラフの形式として平均棒グラフを指定する．このオプションを指定する場合，***`-s`*** オプションは指定できない．

### stat2scrapbox.py

```
./stat2scrapbox.py input_dirs... [-s] (-n | -t)
```

#### 説明

stat\_from\_csv.py が出力した stat ファイルを含むログディレクトリからデータを整形してスループットを Scrapbox のテーブル記法で出力する．

#### 引数

***`input_dirs...`***

&nbsp; &nbsp; stat ファイルを含むログディレクトリ．複数指定する場合はそれらのスループットの平均を取る．

#### オプション

***`-s`***

&nbsp; &nbsp; スループットの標準偏差を出力する．

***`-n`***

&nbsp; &nbsp; 実行したメッセージブローカがスレッドプール方式であることを指定する．このオプションを指定する場合，***`-t`*** オプションは指定できない．

***`-t`***

&nbsp; &nbsp; 実行したメッセージブローカがタイムアウト方式であることを指定する．このオプションを指定する場合，***`-n`*** オプションは指定できない．

### raft_test.sh

```
./raft_test.sh
```

#### 説明

以下の3パターンのスループットを測定し，グラフを生成する．生成されるグラフは，1のデータから1枚と，2,3のデータから1枚の合計2枚である．
1. Raftノード5で，センダ，レシーバともに1,10,20の9組のスループットを測定する．
2. センダ，レシーバともに1で，Raftノード1から10までの10組のスループットを測定する．
3. センダ，レシーバともに10で，Raftノード1から10までの10組のスループットを測定する．

scriptsディレクトリの上の階層のlogディレクトリを削除して再生成するため，注意が必要である．


## 使い方

### autotest\_all.sh の使用例

メッセージブローカとして m-broker-tp を使用し，センダ，レシーバ，メッセージブローカのスレッド数を変更しながら，その組み合わせの回数 autotest.sh を実行する．

```
./autotest_all.sh -b ../m-broker-tp -r 1,10 -s 1,10 -c 100000 -d 3 -l ../log/$(date +%Y%m%d-%H%M%S)/%b-%r-%s-%n-%c-$(date +%Y%m%d-%H%M%S).log -- -n 1-2
```

### 実行結果

センダ，レシーバ，メッセージブローカのスレッド数の条件をオプション引数の範囲で変更し，その組み合わせの数 autotest.sh の出力ファイルが出力される．

```
実際の出力ファイル例
log/20230803-173214/

m-broker-tp-1-1-1-100000-20230803-173214-mpstat.log   m-broker-tp-10-1-1-100000-20230803-173214-mpstat.log
m-broker-tp-1-1-1-100000-20230803-173214-mpstat.pdf   m-broker-tp-10-1-1-100000-20230803-173214-mpstat.pdf
m-broker-tp-1-1-1-100000-20230803-173214-mpstat.tsv   m-broker-tp-10-1-1-100000-20230803-173214-mpstat.tsv
m-broker-tp-1-1-1-100000-20230803-173214.config       m-broker-tp-10-1-1-100000-20230803-173214.config
m-broker-tp-1-1-1-100000-20230803-173214.log          m-broker-tp-10-1-1-100000-20230803-173214.log
m-broker-tp-1-1-2-100000-20230803-173214-mpstat.log   m-broker-tp-10-1-2-100000-20230803-173214-mpstat.log
m-broker-tp-1-1-2-100000-20230803-173214-mpstat.pdf   m-broker-tp-10-1-2-100000-20230803-173214-mpstat.pdf
m-broker-tp-1-1-2-100000-20230803-173214-mpstat.tsv   m-broker-tp-10-1-2-100000-20230803-173214-mpstat.tsv
m-broker-tp-1-1-2-100000-20230803-173214.config       m-broker-tp-10-1-2-100000-20230803-173214.config
m-broker-tp-1-1-2-100000-20230803-173214.log          m-broker-tp-10-1-2-100000-20230803-173214.log
m-broker-tp-1-10-1-100000-20230803-173214-mpstat.log  m-broker-tp-10-10-1-100000-20230803-173214-mpstat.log
m-broker-tp-1-10-1-100000-20230803-173214-mpstat.pdf  m-broker-tp-10-10-1-100000-20230803-173214-mpstat.pdf
m-broker-tp-1-10-1-100000-20230803-173214-mpstat.tsv  m-broker-tp-10-10-1-100000-20230803-173214-mpstat.tsv
m-broker-tp-1-10-1-100000-20230803-173214.config      m-broker-tp-10-10-1-100000-20230803-173214.config
m-broker-tp-1-10-1-100000-20230803-173214.log         m-broker-tp-10-10-1-100000-20230803-173214.log
m-broker-tp-1-10-2-100000-20230803-173214-mpstat.log  m-broker-tp-10-10-2-100000-20230803-173214-mpstat.log
m-broker-tp-1-10-2-100000-20230803-173214-mpstat.pdf  m-broker-tp-10-10-2-100000-20230803-173214-mpstat.pdf
m-broker-tp-1-10-2-100000-20230803-173214-mpstat.tsv  m-broker-tp-10-10-2-100000-20230803-173214-mpstat.tsv
m-broker-tp-1-10-2-100000-20230803-173214.config      m-broker-tp-10-10-2-100000-20230803-173214.config
m-broker-tp-1-10-2-100000-20230803-173214.log         m-broker-tp-10-10-2-100000-20230803-173214.log
```

### autotest.sh の使用例

メッセージブローカとして m-broker-tp を使用し，センダのプロセス数1，レシーバのプロセス数1，メッセージブローカのスレッド数10，メッセージ数100000で実験を行い，メッセージブローカのログファイルを出力し，mpstat の結果をグラフに出力する．

```
./autotest.sh -b ../m-broker-tp -r 1 -s 1 -c 100000 -l ./log/20230719-163604/m-broker-tp-1-1-1-100000-20230719-163604 -- -n 10
```

### 

### mpstat を用いた CPU Usage の測定

#### ログを取る

mpstat を用いてログをファイルに出力する．

例: 1秒毎にすべての CPU の情報を出力する．

```
mpstat -P ALL 1 > output_file_name
```

上記コマンドを実行中に，測定したいプロセスを実行する．実行が終了したら，Ctrl + C で終了する．

#### グラフに出力する

下記のコマンドでログの整形を行う．

```
./log2csv.sh input_file
```
input\_file は mpstat で出力されたファイルである．input\_file に .csv をつけたファイルが出力される．

下記のコマンドでグラフを出力する．

```
./plot_from_mpstat_log.py input_file output_file cores
```
input\_file は log2csv.sh で出力されたファイルである．output\_file に .pdf をつけたファイルが出力される．

### raft_test.sh の使用例

このスクリプトはscriptsディレクトリ内で使用する．スクリプト内で，カレントディレクトリからの相対的な位置にあるファイルを操作しているため，scriptsディレクトリ以外から実行すると正常に動作しない．
```
# /scripts
./raft_test.sh
```

### 実行結果

```
# 実際の出力ファイル
# /scripts
ls ../log
10_clients_n_nodes
1_and_10_clients_n_nodes.pdf
1_clients_n_nodes
n_clients_5_nodes
n_clients_5_nodes.pdf
```
ファイルとディレクトリの説明
1. 1_clients_n_nodes
   + 1.1センダ，レシーバともに1で，Raftノード1から10までの10組のデータを含むディレクトリ
2. 10_clients_n_nodes
   + センダ，レシーバともに10で，Raftノード1から10までの10組のデータを含むディレクトリ
3. 1_and_10_clients_n_nodes.pdf
   + 1,2のデータから生成された，メッセージブローカのスループットを表すグラフ
4. n_clients_5_nodes
   + Raftノード5で，センダ，レシーバともに1,10,20の9組のデータを含むディレクトリ
5. n_clients_5_nodes.pdf
   + 4のデータから生成された，メッセージブローカのスループットを表すグラフ
