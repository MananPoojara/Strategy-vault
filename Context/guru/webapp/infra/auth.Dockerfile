FROM node:24-bookworm-slim
WORKDIR /app/auth
COPY --chown=node:node webapp/auth/package*.json ./
RUN --mount=type=secret,id=build_ca if [ -s /run/secrets/build_ca ]; then export NODE_EXTRA_CA_CERTS=/run/secrets/build_ca; fi; npm ci --omit=dev --no-fund --no-audit
COPY --chown=node:node webapp/auth/src ./src
COPY --chown=node:node webapp/db /app/db
USER node
CMD ["node", "src/server.mjs"]
