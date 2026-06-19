# Настройка Traefik с использованием Unix Socket (Docker)

В данном руководстве показано, как запустить `worker` и `traefik` в Docker, настроив связь между ними через Unix-сокет, а также настроить SSL/TLS сертификаты.

## Docker Compose (`docker-compose.yml`)

Для Traefik мы прокинем конфигурацию и порты. Если вы используете автоматические сертификаты, потребуется Volume для сохранения `acme.json`. Если ручные — папка с файлами сертификатов.

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
      - "--providers.file.filename=/etc/traefik/dynamic.yml"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
      # Настройка для автоматических сертификатов (Let's Encrypt):
      # - "--certificatesresolvers.myresolver.acme.tlschallenge=true"
      # - "--certificatesresolvers.myresolver.acme.email=your-email@example.com"
      # - "--certificatesresolvers.myresolver.acme.storage=/letsencrypt/acme.json"
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - sockets_vol:/sockets
      - ./dynamic.yml:/etc/traefik/dynamic.yml
      # Для автоматических сертификатов раскомментируйте:
      # - ./letsencrypt:/letsencrypt
      # Для ручных сертификатов раскомментируйте:
      # - ./certs:/certs
    depends_on:
      - worker

volumes:
  sockets_vol:
```

## Динамическая конфигурация Traefik (`dynamic.yml`)

### Вариант 1: Автоматические сертификаты (Let's Encrypt)
Раскомментируйте настройки `certificatesresolvers` в `docker-compose.yml`, а в `dynamic.yml` укажите использование этого резолвера (`certResolver: myresolver`) для вашего роутера.

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

### Вариант 2: Ручные сертификаты
Сертификаты прописываются в блоке `tls.certificates`. Убедитесь, что пути в `certFile` и `keyFile` соответствуют папке, которую вы примонтировали в `docker-compose.yml` (`/certs`).

```yaml
http:
  routers:
    worker-router:
      rule: "Host(`example.com`) && PathPrefix(`/apiws`)"
      service: worker-service
      entryPoints:
        - websecure
      # Включаем TLS без резолвера (будет использовать дефолтные загруженные сертификаты)
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

### Решение возможных проблем с правами (Permissions)
Если Traefik не может подключиться к сокету из-за прав доступа:
Запустите оба контейнера (`worker` и `traefik`) от одного и того же пользователя с помощью директивы `user: "1000:1000"` в конфигурации каждого сервиса в файле `docker-compose.yml`.
