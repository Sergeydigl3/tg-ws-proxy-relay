# Настройка Caddy с использованием Unix Socket (Docker)

В данном руководстве показано, как запустить `worker` и веб-сервер `caddy` в Docker, настроив связь между ними через Unix-сокет, а также настроить SSL/TLS сертификаты.

## Docker Compose (`docker-compose.yml`)

Для общения через Unix-сокет необходимо создать общую директорию (Shared Volume). Чтобы Caddy мог автоматически получать и продлевать сертификаты, мы также монтируем `caddy_data`.

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
      # Папка для сохранения автоматически полученных сертификатов (Let's Encrypt)
      - caddy_data:/data
      # Раскомментируйте строку ниже, если используете свои (ручные) сертификаты:
      # - ./certs:/etc/caddy/certs
    depends_on:
      - worker

volumes:
  sockets_vol:
  caddy_data:
```

## Конфигурация Caddy (`Caddyfile`)

Caddy автоматически управляет заголовками для WebSocket.

### Вариант 1: Автоматические сертификаты (Let's Encrypt)
По умолчанию Caddy сам запросит сертификат для вашего домена (убедитесь, что A-запись DNS настроена на сервер). Больше ничего писать не нужно.

```caddyfile
example.com {
    route /apiws* {
        reverse_proxy unix//sockets/worker.sock
    }
}
```

### Вариант 2: Ручные сертификаты
Если вы используете собственные сертификаты (например, корпоративные или от Cloudflare), укажите пути к ним с помощью директивы `tls`. Убедитесь, что вы примонтировали папку с сертификатами в `docker-compose.yml`.

```caddyfile
example.com {
    # Путь к публичному сертификату и приватному ключу внутри контейнера
    tls /etc/caddy/certs/cert.pem /etc/caddy/certs/key.pem
    
    route /apiws* {
        reverse_proxy unix//sockets/worker.sock
    }
}
```

### Решение возможных проблем с правами (Permissions)
Сокет создается от имени пользователя, запускающего `worker`. Если Caddy выдает ошибки прав доступа при попытке чтения сокета, запустите оба контейнера (`worker` и `caddy`) от одного и того же пользователя с помощью директивы `user: "1000:1000"` в `docker-compose.yml`.
