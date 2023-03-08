openssl genrsa -out ./key.pem
openssl req -new -key ./key.pem -out csr.pem