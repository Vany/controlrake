package qrcoder

import (
	"context"
	"fmt"
	"github.com/mdp/qrterminal/v3"
	httpserver_api "github.com/vany/controlrake/src/httpserver/api"
	. "github.com/vany/pirog"
	"net"
	"os"
	"regexp"
)

// QrCoder - draw where to connect on app start.
type QrCoder struct {
	HTTPServer httpserver_api.HTTPServer `inject:"HTTPServer"`
	// later, draw qrcode in//	ObsBrowser obsbrowser_api.ObsBrowser `inject:"ObsBrowser"`
}

func New() *QrCoder { return new(QrCoder) }

var v4Re = regexp.MustCompile(`^\d+\.\d+\.\d+\.\d+`)

func (q *QrCoder) Run(ctx context.Context) error {
	ifs := MUST2(net.Interfaces())

	ifs = GREP(ifs, func(i net.Interface) bool {
		if i.HardwareAddr == nil {
			return false
		}
		return (i.Flags & net.FlagRunning) != 0
	})

	ass := []string{}
	for _, i := range ifs {
		addrs, _ := i.Addrs()
		addrs = GREP(addrs, func(in net.Addr) bool { return v4Re.MatchString(in.String()) })
		ass = append(ass, MAP(addrs, func(in net.Addr) string {
			return v4Re.FindString(in.String())
		})...)
	}

	println("Please connect to:")
	for _, addr := range ass {
		conn := q.HTTPServer.GetBaseUrl(addr)
		fmt.Println(" => " + conn)
		qrterminal.Generate(conn, qrterminal.M, os.Stdout)
	}
	return nil
}
