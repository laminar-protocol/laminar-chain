# Based from https://github.com/paritytech/substrate/blob/master/.maintain/Dockerfile

FROM phusion/baseimage:0.11 as builder
LABEL maintainer="hello@laminar.one"
LABEL description="This is the build stage for Laminar Chain Node. Here we create the binary."

ENV DEBIAN_FRONTEND=noninteractive

ARG PROFILE=release
WORKDIR /laminar

COPY . /laminar

RUN apt-get update && \
	apt-get dist-upgrade -y -o Dpkg::Options::="--force-confold" && \
	apt-get install -y cmake pkg-config libssl-dev git clang

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
	export PATH="$PATH:$HOME/.cargo/bin" && \
	rustup toolchain install nightly-2020-09-27 && \
	rustup target add wasm32-unknown-unknown --toolchain nightly-2020-09-27 && \
	rustup default stable && \
	cargo build "--$PROFILE"

# ===== SECOND STAGE ======

FROM phusion/baseimage:0.11
LABEL maintainer="hello@laminar.one"
LABEL description="This is the 2nd stage: a very small image where we copy the Laminar Chain Node binary."
ARG PROFILE=release

RUN mv /usr/share/ca* /tmp && \
	rm -rf /usr/share/*  && \
	mv /tmp/ca-certificates /usr/share/ && \
	useradd -m -u 1000 -U -s /bin/sh -d /laminar laminar && \
	mkdir -p /laminar/.local/share/laminar && \
	chown -R laminar:laminar /laminar/.local && \
	ln -s /laminar/.local/share/laminar /data

COPY --from=builder /laminar/target/$PROFILE/laminar /usr/local/bin

# checks
RUN ldd /usr/local/bin/laminar && \
	/usr/local/bin/laminar --version

# Shrinking
RUN rm -rf /usr/lib/python* && \
	rm -rf /usr/bin /usr/sbin /usr/share/man

USER laminar
EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/laminar"]
