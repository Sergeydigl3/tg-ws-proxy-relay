# Telegram MTProto WebSocket Proxy

В RU-зоне на вашем сервере необходимо развернуть **`tg-ws-proxy`**. 

⚠️ **Важно:** Сам Rust-воркер при этом должен находиться на другом сервере **за пределами RU-зоны** (в стране, где Telegram работает без ограничений). 

Локальный `tg-ws-proxy` маскирует клиентский трафик (FakeTLS) и перенаправляет его в ваш удаленный Worker через WebSocket, обеспечивая надежный обход блокировок. Работает как самостоятельно, так и за балансировщиком.

### Пример с Traefik (`docker-compose.yml`)
Пример настройки `tg-ws-proxy` за Traefik (TCP-роутинг + passthrough):

```yaml
services:
  # ─────────────────────────────────────────────────────────────
  #  tg-ws-proxy  —  MTProto-прокси с FakeTLS
  #
  #  Как это работает:
  #    Traefik слушает :443 (TCP), пикает SNI без терминации TLS
  #    (passthrough), матчит домен и форвардит поток на порт 8446
  #    контейнера + добавляет PROXY Protocol заголовок.
  #    Прокси принимает PROXY Protocol и обрабатывает fake-TLS сам.
  #
  #  Ссылка для подключения будет в логах контейнера:
  #    docker logs tg-ws-proxy-b 2>&1 | grep 'tg://proxy'
  tg-ws-proxy-b:
    build:
      context: https://github.com/Flowseal/tg-ws-proxy.git#5bc5001c4dd5d7833913041b31d7663d6ccc66cc

    container_name: tg-ws-proxy-b
    restart: unless-stopped

    command: >-
      --pool-size 15
      --no-cfproxy
      --port 8446
      --host 0.0.0.0
      --cfproxy-worker-domain ${TG_CFPROXY_WORKER_DOMAIN}
      --fake-tls-domain ${TG_PROXY_DOMAIN}
      --proxy-protocol
      --secret ${TG_WS_PROXY_SECRET}

    environment:
      # Генерация секрета: openssl rand -hex 16
      TG_WS_PROXY_SECRET: ${TG_WS_PROXY_SECRET}
      TG_PROXY_DOMAIN: ${TG_PROXY_DOMAIN}
      TG_CFPROXY_WORKER_DOMAIN: ${TG_CFPROXY_WORKER_DOMAIN}
      # TG_WS_PROXY_DC_IPS: 0:0.0.0.0

    networks:
      - traefik

    labels:
      - "traefik.enable=true"
      - "traefik.tcp.routers.tg-proxy-b.entrypoints=websecure"
      - "traefik.tcp.routers.tg-proxy-b.rule=HostSNI(`${TG_PROXY_DOMAIN}`)"
      - "traefik.tcp.routers.tg-proxy-b.tls.passthrough=true"
      - "traefik.tcp.services.tg-proxy-b.loadbalancer.server.port=8446"
      - "traefik.tcp.services.tg-proxy-b.loadbalancer.proxyprotocol.version=1"

networks:
  traefik:
    external: true
```

### Пример конфигурации окружения (`.env`)

Создайте файл `.env` рядом с вашим `docker-compose.yml` и заполните его следующими значениями:

```env
# Ваш секрет (генерируется командой: openssl rand -hex 16)
# Например: ee8dc66f3e74eae2ccce0d7752418ab023
TG_WS_PROXY_SECRET=your_generated_secret_here

# Домен для FakeTLS (домен, которым будет маскироваться трафик, например: google.com, yandex.ru)
TG_PROXY_DOMAIN=google.com

# Домен вашего Rust-воркера (через который пойдет вебсокет-трафик)
TG_CFPROXY_WORKER_DOMAIN=wsstg.yourdomain.ru
```

---

## Подробные инструкции по настройке Worker-a принимающего вебсокет-соединения за реверс-прокси  (для тлс)
* 📘 [Настройка Traefik](worker/docs/traefik.md)
* 📗 [Настройка NGINX](worker/docs/nginx.md)
* 📙 [Настройка Caddy](worker/docs/caddy.md)
