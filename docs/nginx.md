# Настройка NGINX с использованием Unix Socket (Docker)

В данном руководстве показано, как запустить `worker` и `nginx` в Docker, настроив связь между ними через Unix-сокет, а также настроить SSL/TLS.

## Docker Compose (`docker-compose.yml`)

Для использования SSL необходимо опубликовать порт 443 и прокинуть директорию с сертификатами в NGINX.

```yaml
version: "3.8"

services:
  worker:
    image: ghcr.io/sergeydigl3/tg-ws-proxy-relay:master
    # Запускаем с прослушиванием unix-сокета в общей папке
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
      # Папка для сертификатов (своих или сгенерированных извне, например через Certbot)
      - ./certs:/etc/nginx/certs
    depends_on:
      - worker

volumes:
  sockets_vol:
```

## Конфигурация NGINX (`nginx.conf`)

### Вариант 1: Ручные сертификаты
В конфигурации необходимо настроить серверный блок на порту 443 и указать пути к сертификатам. Запросы с HTTP перенаправляются на HTTPS.

```nginx
events {}

http {
    # Редирект с HTTP на HTTPS
    server {
        listen 80;
        server_name example.com;
        return 301 https://$host$request_uri;
    }

    server {
        listen 443 ssl;
        server_name example.com;

        # Пути к вашим ручным сертификатам (из папки ./certs)
        ssl_certificate /etc/nginx/certs/cert.pem;
        ssl_certificate_key /etc/nginx/certs/key.pem;

        location /apiws {
            # Проксирование на Unix-сокет
            proxy_pass http://unix:/sockets/worker.sock;
            
            # Настройки для WebSocket
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "Upgrade";
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            
            # Увеличенные таймауты
            proxy_read_timeout 86400s;
            proxy_send_timeout 86400s;
        }
    }
}
```

### Вариант 2: Автоматические сертификаты (Let's Encrypt / Certbot)
NGINX сам по себе не умеет запрашивать сертификаты. Если вы хотите автоматизировать процесс:
1. Вы можете использовать сторонний `certbot` (через cron на сервере или в виде отдельного Docker-контейнера). 
2. `certbot` сохраняет сертификаты по пути `/etc/letsencrypt/live/example.com/...`.
3. Примонтируйте директорию letsencrypt к контейнеру nginx:
   ```yaml
       volumes:
         - /etc/letsencrypt:/etc/letsencrypt:ro
   ```
4. В `nginx.conf` укажите пути сгенерированные Certbot:
   ```nginx
        ssl_certificate /etc/letsencrypt/live/example.com/fullchain.pem;
        ssl_certificate_key /etc/letsencrypt/live/example.com/privkey.pem;
   ```

*(Также отличной альтернативой для автоматических сертификатов с NGINX является образ **Nginx Proxy Manager**, который предоставляет графический интерфейс).*

### Решение возможных проблем с правами (Permissions)
Если NGINX выдает ошибку `13: Permission denied` при доступе к сокету:
Запустите оба контейнера (`worker` и `nginx`) от одного и того же пользователя (добавив `user: "1000:1000"` в `docker-compose.yml`).
