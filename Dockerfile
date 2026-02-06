FROM debian:bookworm-slim

ARG TARGETARCH

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /bin/bash monphare

COPY ${TARGETARCH}/monphare /usr/local/bin/monphare
RUN chmod +x /usr/local/bin/monphare

USER monphare
WORKDIR /workspace

ENTRYPOINT ["monphare"]
