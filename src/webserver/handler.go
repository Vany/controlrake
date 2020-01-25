package webserver

import (
	"fmt"
	"log"
	"net/http"
)

func logging(logger *log.Logger) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			defer func() {
				s := fmt.Sprintf("%#v", w.Header())
				logger.Println("HTTP: ", r.Method, r.URL.Path, r.RemoteAddr, s)
			}()
			next.ServeHTTP(w, r)
		})
	}
}
