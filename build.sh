docker build -t chushi0/rustweb/web-bss -f ./docker/web-bss.dockerfile .
docker build -t chushi0/rustweb/game-backend -f ./docker/game-backend.dockerfile .
docker build -t chushi0/rustweb/web-cronjob -f ./docker/web-cronjob.dockerfile .
# docker build -t chushi0/rustweb/web-fe -f ./docker/web-fe.dockerfile .