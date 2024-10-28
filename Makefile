build-prod:
	cargo build -r
	pm2 restart prod.blast.json

deploy-prod:
		rsync -avhzL --delete \
    				--no-perms --no-owner --no-group \
    				--exclude .git \
    				--filter=":- .gitignore" \
    				. ubuntu@54.226.112.134:/home/ubuntu/bcat-mm