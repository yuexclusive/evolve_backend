FROM alpine
ARG target
ARG ip
RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.ustc.edu.cn/g' /etc/apk/repositories
RUN apk update && apk add tzdata
RUN ln /usr/share/zoneinfo/Asia/Shanghai /etc/localtime
RUN echo "Asia/Shanghai" >/etc/timezone
WORKDIR /app
COPY ./target/${target}/release/evolve_backend .
COPY config.toml .
COPY .env .
COPY log4rs.yml .
COPY static static
RUN sed -i "s/127.0.0.1/${ip}/g" config.toml
RUN sed -i "s/127.0.0.1/${ip}/g" .env
CMD ["/app/evolve_backend"]
