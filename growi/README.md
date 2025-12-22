# GROWI docker definition

GROWI 公式の docker compose テンプレート growi-docker-compose を
submodule として登録しています。
git clone, pull しただけでは更新されません。

```sh
# 1回でよい
git submodule init
# 更新があったら毎回
git submodule update

# 同時にやる
git submodule update --init

# clone 時に一緒にやってしまう
git clone --recursive
```

なおあまりに面倒な上に update を忘れると壊れるので以下の設定をしておけば解決。

```sh
git config --global submodule.recurse true
```
