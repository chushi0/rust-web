# deploy mysql
docker run --name rustweb.chushi0.mysql -d --network rustweb-internal -v /usr/docker-data/mysql:/var/lib/mysql mysql:latest

# web-bff
docker run --name rustweb.chushi0.web-bff -d --network rustweb-internal -p 8080:8080 -p 3000:3000 -e RUST_WEB_DB_USERNAME=<username> -e RUST_WEB_DB_PASSWORD=<password> chushi0/rustweb/web-bff:latest
# web-cronjob
docker run --name rustweb.chushi0.web-cronjob -d --network rustweb-internal -e RUST_WEB_DB_USERNAME=<username> -e RUST_WEB_DB_PASSWORD=<password> chushi0/rustweb/web-cronjob:latest
# game-backend
docker run --name rustweb.chushi0.game-backend -d --network rustweb-internal chushi0/rustweb/game-backend:latest