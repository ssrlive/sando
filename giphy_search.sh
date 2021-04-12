crt_file="domain.crt"
domain="proxy.tokio.sando"
api_key="GchRU9ycdwk7nPJv8NeCtPZZviLFhTCP"

for (( ; ; ))
do
  read -r -p "search for: " query
  # Replace the space ' ' by "%20"
  query=$(printf "%s\n" "$query" | sed 's/ /%20/g' )
  read -r -p "max number of items: " limit
  # https://developers.giphy.com/explorer
  curl -vp --proxy "https://$domain:7878" --proxy-cacert $crt_file \
    "https://api.giphy.com/v1/gifs/search?q=${query}&api_key=${api_key}&limit=${limit}" \
    | python -m json.tool

  read -r -p "Quit? [y/N] " response
  case "$response" in
  [yY][eE][sS] | [yY])
    break
    ;;
  *) ;;
  esac
done

