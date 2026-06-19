# Настройка Traefik с использованием Unix Socket (Docker)

В данном руководстве показано, как запустить `worker` и `traefik` в Docker через Unix-сокет. Во всех примерах Traefik будет следить за директорией `traefik-dynamic` и автоматически подхватывать файлы конфигурации.

---

## Вариант 1: Автоматические сертификаты (Let's Encrypt)

### Docker Compose (`docker-compose.yml`)
Мы настраиваем `certificatesresolvers` (в данном примере называется `myresolver`) и создаем volume для файла `acme.json`, чтобы не исчерпать лимиты Let's Encrypt при перезапусках.

```yaml
version: "3.8"

services:
  worker:
    image: ghcr.io/sergeydigl3/tg-ws-proxy-relay:master
    command: ["--listen-type", "unix", "--listen-addr", "/sockets/worker.sock"]
    volumes:
      - sockets_vol:/sockets
    restart: unless-stopped

  traefik:
    image: traefik:v3.0
    command:
      - "--providers.docker=false"
      - "--providers.file.directory=/etc/traefik/dynamic"
      - "--providers.file.watch=true"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
      - "--certificatesresolvers.myresolver.acme.tlschallenge=true"
      - "--certificatesresolvers.myresolver.acme.email=your-email@example.com"
      - "--certificatesresolvers.myresolver.acme.storage=/letsencrypt/acme.json"
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - sockets_vol:/sockets
      - ./traefik-dynamic:/etc/traefik/dynamic:ro
      - ./letsencrypt:/letsencrypt
    depends_on:
      - worker

volumes:
  sockets_vol:
```

### Динамическая конфигурация (`traefik-dynamic/worker.yml`)
Здесь мы указываем, что роутер должен использовать `certResolver: myresolver`.

```yaml
http:
  routers:
    worker-router:
      rule: "Host(`example.com`) && PathPrefix(`/apiws`)"
      service: worker-service
      entryPoints:
        - websecure
      tls:
        certResolver: myresolver

  services:
    worker-service:
      loadBalancer:
        servers:
          - url: "http://unix:///sockets/worker.sock"
```

---

## Вариант 2: Ручные (свои) сертификаты

### Docker Compose (`docker-compose.yml`)
Здесь мы монтируем папку `./certs`, в которой лежат ваши файлы сертификатов (например, `cert.pem` и `key.pem`). Настройки `certificatesresolvers` не нужны.

```yaml
version: "3.8"

services:
  worker:
    image: ghcr.io/sergeydigl3/tg-ws-proxy-relay:master
    command: ["--listen-type", "unix", "--listen-addr", "/sockets/worker.sock"]
    volumes:
      - sockets_vol:/sockets
    restart: unless-stopped

  traefik:
    image: traefik:v3.0
    command:
      - "--providers.docker=false"
      - "--providers.file.directory=/etc/traefik/dynamic"
      - "--providers.file.watch=true"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - sockets_vol:/sockets
      - ./traefik-dynamic:/etc/traefik/dynamic:ro
      - ./certs:/certs:ro
    depends_on:
      - worker

volumes:
  sockets_vol:
```

### Динамическая конфигурация (`traefik-dynamic/worker.yml`)
Мы указываем пути к сертификатам в блоке `tls.certificates`, а у самого роутера просто включаем `tls: {}`.

```yaml
http:
  routers:
    worker-router:
      rule: "Host(`example.com`) && PathPrefix(`/apiws`)"
      service: worker-service
      entryPoints:
        - websecure
      tls: {}

  services:
    worker-service:
      loadBalancer:
        servers:
          - url: "http://unix:///sockets/worker.sock"

tls:
  certificates:
    - certFile: "/certs/cert.pem"
      keyFile: "/certs/key.pem"
```

---

## Решение возможных проблем с правами (Permissions)
Запустите оба контейнера (`worker` и `traefik`) от одного и того же пользователя с помощью директивы `user: "1000:1000"` в `docker-compose.yml`, если Traefik выдает ошибки прав доступа при чтении сокета.
