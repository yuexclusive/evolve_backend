FROM nginx:alpine
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
COPY default.conf /etc/nginx/conf.d/default.conf
RUN rm -fr /var/log/nginx/*

RUN echo "#! /bin/sh" >>start.sh
# RUN echo "nginx -g 'daemon off;'" >> start.sh
RUN echo "nginx" >>start.sh
RUN echo "nohup ./evolve_backend 2>&1 > /dev/null &" >>start.sh
RUN echo "sh" >>start.sh
RUN chmod u+x start.sh

CMD ["/app/start.sh"]
