# Stage 1: Build the SPA
FROM oven/bun:1 AS builder

WORKDIR /app
COPY . .

RUN bun install
RUN cd apps/app && PORT=3000 VITE_DEFAULT_REGISTRY_URLS=https://web.registry.beta.withadi.dev bunx vite build

# Stage 2: Serve with nginx
FROM nginx:alpine

# SPA fallback: serve index.html for all routes
RUN printf 'server {\n\
    listen 8080;\n\
    root /usr/share/nginx/html;\n\
    index index.html;\n\
    location / {\n\
        try_files $uri $uri/ /index.html;\n\
    }\n\
    location ~* \\.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2|ttf|eot)$ {\n\
        expires 1y;\n\
        add_header Cache-Control "public, immutable";\n\
    }\n\
}\n' > /etc/nginx/conf.d/default.conf

COPY --from=builder /app/apps/app/dist/ /usr/share/nginx/html/
COPY --from=builder /app/vendor/ /usr/share/nginx/html/vendor/

EXPOSE 8080

CMD ["nginx", "-g", "daemon off;"]
