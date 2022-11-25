##
## To run the validation:
##
##     docker image build --build-arg HEIGHT=10010 --progress=plain .
##
FROM ubuntu

RUN apt update && apt install --yes git golang wget openssl ca-certificates
RUN git clone --branch ielashi/compute_scripts --single-branch https://github.com/dfinity/bitcoin-canister.git

RUN go install github.com/in3rsha/bitcoin-utxo-dump@5723696e694ebbfe52687f51e7fc0ce62ba43dc8
RUN wget https://bitcoin.org/bin/bitcoin-core-22.0/bitcoin-22.0-x86_64-linux-gnu.tar.gz
RUN tar xzvf bitcoin-22.0-x86_64-linux-gnu.tar.gz 

ARG HEIGHT
ENV BITCOIN_DIR=/bitcoin-22.0
ENV HEIGHT=${HEIGHT}

RUN bitcoin-canister/bootstrap/step_1.sh ${BITCOIN_DIR} ${HEIGHT}
RUN bitcoin-canister/bootstrap/check_chaintip.sh ${BITCOIN_DIR}
RUN bitcoin-canister/bootstrap/step_2.sh ${BITCOIN_DIR} ${HEIGHT}
RUN bitcoin-canister/bootstrap/step_3.sh ${BITCOIN_DIR} ${HEIGHT}
RUN bitcoin-canister/bootstrap/step_4.sh ${HEIGHT}
RUN sha256sum unstable_blocks block_headers utxodump.csv
