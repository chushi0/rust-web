docker build -t chushi0/rustweb/web-bff -f ./docker/web-bff.dockerfile .
docker build -t chushi0/rustweb/game-backend -f ./docker/game-backend.dockerfile .
docker build -t chushi0/rustweb/web-cronjob -f ./docker/web-cronjob.dockerfile .
# docker build -t chushi0/rustweb/web-fe -f ./docker/web-fe.dockerfile .
docker build -t chushi0/rustweb/core-rpc -f ./docker/core-rpc.dockerfile .