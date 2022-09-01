#FROM anatolse/myrepo:diadro
#EXPOSE 8081
#CMD ["/dserver", "/docs"]

FROM ghcr.io/cross-rs/x86_64-pc-windows-gnu

RUN dpkg --add-architecture arm64 && \
    apt-get update

CMD ["/bin/bash"]