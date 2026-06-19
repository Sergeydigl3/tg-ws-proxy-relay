# Настройка Caddy с использованием Unix Socket (Docker)

В данном руководстве показано, как запустить `worker` и веб-сервер `caddy` в Docker через Unix-сокет.

---

## Вариант 1: Автоматические сертификаты (Let's Encrypt)

Этот вариант автоматически получает и продлевает SSL сертификат для вашего домена (убедитесь, что A-запись DNS настроена на сервер).

### Docker Compose (`docker-compose.yml`)
Мы монтируем общую директорию `sockets_vol` для общения контейнеров и `caddy_data` для сохранения полученных сертификатов.

```yaml
version: "3.8"

services:
  worker:
    image: ghcr.io/sergeydigl3/tg-ws-proxy-relay:master
    command: ["--listen-type", "unix", "--listen-addr", "/sockets/worker.sock"]
    volumes:
      - sockets_vol:/sockets
    restart: unless-stopped

  caddy:
    image: caddy:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - sockets_vol:/sockets
      - ./Caddyfile:/etc/caddy/Caddyfile
      - caddy_data:/data
    depends_on:
      - worker

volumes:
  sockets_vol:
  caddy_data:
```

### Конфигурация (`Caddyfile`)
Caddy автоматически управляет заголовками WebSocket. Просто укажите ваш домен:

```caddyfile
example.com {
    route /apiws* {
        reverse_proxy unix//sockets/worker.sock
    }
}
```

---

## Вариант 2: Ручные (свои) сертификаты

Используйте этот вариант, если у вас есть собственные сертификаты (например, от Cloudflare или корпоративные).

### Docker Compose (`docker-compose.yml`)
Мы монтируем папку `./certs`, в которой лежат ваши файлы сертификатов (например, `cert.pem` и `key.pem`).

```yaml
version: "3.8"

services:
  worker:
    image: ghcr.io/sergeydigl3/tg-ws-proxy-relay:master
    command: ["--listen-type", "unix", "--listen-addr", "/sockets/worker.sock"]
    volumes:
      - sockets_vol:/sockets
    restart: unless-stopped

  caddy:
    image: caddy:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - sockets_vol:/sockets
      - ./Caddyfile:/etc/caddy/Caddyfile
      - ./certs:/etc/caddy/certs:ro
    depends_on:
      - worker

volumes:
  sockets_vol:
```

### Конфигурация (`Caddyfile`)
Добавьте директиву `tls` с путями к сертификату и ключу:

```caddyfile
example.com {
    tls /etc/caddy/certs/cert.pem /etc/caddy/certs/key.pem
    
    route /apiws* {
        reverse_proxy unix//sockets/worker.sock
    }
}
```

---

## Решение возможных проблем с правами (Permissions)
Сокет создается от имени пользователя, запускающего `worker`. Если Caddy выдает ошибки прав доступа при попытке чтения сокета, запустите оба контейнера (`worker` и `caddy`) от одного и того же пользователя с помощью директивы `user: "1000:1000"` в каждом сервисе файла `docker-compose.yml`.
