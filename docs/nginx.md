# Настройка NGINX с использованием Unix Socket (Docker)

В данном руководстве показано, как запустить `worker` и `nginx` в Docker через Unix-сокет.

---

## Вариант 1: Автоматические сертификаты (Let's Encrypt / Certbot)

NGINX сам по себе не умеет запрашивать сертификаты, поэтому для автоматизации мы используем внешний контейнер `certbot` (или запуск на хосте).
> *Примечание: Если вам нужно полностью готовое "всё в одном" решение для Nginx с автоматическим SSL, рассмотрите [Nginx Proxy Manager](https://nginxproxymanager.com/).*

### Docker Compose (`docker-compose.yml`)
Мы монтируем директорию `/etc/letsencrypt`, куда сторонний процесс (или контейнер) `certbot` складывает готовые сертификаты.

```yaml
version: "3.8"

services:
  worker:
    image: ghcr.io/sergeydigl3/tg-ws-proxy-relay:master
    command: ["--listen-type", "unix", "--listen-addr", "/sockets/worker.sock"]
    volumes:
      - sockets_vol:/sockets
    restart: unless-stopped

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - sockets_vol:/sockets
      - ./nginx.conf:/etc/nginx/nginx.conf
      # Папка, сгенерированная Certbot-ом
      - ./letsencrypt:/etc/letsencrypt:ro
    depends_on:
      - worker

volumes:
  sockets_vol:
```

### Конфигурация (`nginx.conf`)
Указываем пути к автоматическим сертификатам от Certbot.

```nginx
events {}

http {
    server {
        listen 80;
        server_name example.com;
        return 301 https://$host$request_uri;
    }

    server {
        listen 443 ssl;
        server_name example.com;

        # Пути к сертификатам, сгенерированным Certbot
        ssl_certificate /etc/letsencrypt/live/example.com/fullchain.pem;
        ssl_certificate_key /etc/letsencrypt/live/example.com/privkey.pem;

        location /apiws {
            proxy_pass http://unix:/sockets/worker.sock;
            
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "Upgrade";
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            
            proxy_read_timeout 86400s;
            proxy_send_timeout 86400s;
        }
    }
}
```

---

## Вариант 2: Ручные (свои) сертификаты

### Docker Compose (`docker-compose.yml`)
Мы монтируем папку `./certs`, в которой лежат ваши файлы сертификатов (`cert.pem` и `key.pem`).

```yaml
version: "3.8"

services:
  worker:
    image: ghcr.io/sergeydigl3/tg-ws-proxy-relay:master
    command: ["--listen-type", "unix", "--listen-addr", "/sockets/worker.sock"]
    volumes:
      - sockets_vol:/sockets
    restart: unless-stopped

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - sockets_vol:/sockets
      - ./nginx.conf:/etc/nginx/nginx.conf
      # Папка для ваших ручных сертификатов
      - ./certs:/etc/nginx/certs:ro
    depends_on:
      - worker

volumes:
  sockets_vol:
```

### Конфигурация (`nginx.conf`)
Указываем пути к вашим ручным сертификатам.

```nginx
events {}

http {
    server {
        listen 80;
        server_name example.com;
        return 301 https://$host$request_uri;
    }

    server {
        listen 443 ssl;
        server_name example.com;

        # Пути к вашим ручным сертификатам
        ssl_certificate /etc/nginx/certs/cert.pem;
        ssl_certificate_key /etc/nginx/certs/key.pem;

        location /apiws {
            proxy_pass http://unix:/sockets/worker.sock;
            
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "Upgrade";
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            
            proxy_read_timeout 86400s;
            proxy_send_timeout 86400s;
        }
    }
}
```

---

## Решение возможных проблем с правами (Permissions)
Если NGINX выдает ошибку `13: Permission denied` при доступе к сокету:
Запустите оба контейнера (`worker` и `nginx`) от одного и того же пользователя (добавив `user: "1000:1000"` в `docker-compose.yml`).
