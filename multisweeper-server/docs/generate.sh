#!fish

asyncapi generate fromTemplate ./asyncapi.json @asyncapi/html-template --param version=3.0.0 singleFile=true --output ./web --force-write && \
python3 -m http.server --directory ./web 8040