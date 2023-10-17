package youtube

import (
	"context"
	"encoding/json"
	"fmt"
	app2 "github.com/vany/controlrake/src/app"
	. "github.com/vany/pirog"
	"golang.org/x/oauth2"
	"golang.org/x/oauth2/google"
	"google.golang.org/api/option"
	"google.golang.org/api/youtube/v3"
	"net/http"
	"os"
	"os/exec"
	"runtime"
	"time"
)

// openURL opens a browser window to the specified location.
// This code originally appeared at:
//
//	http://stackoverflow.com/questions/10377243/how-can-i-launch-a-process-that-is-not-a-file-in-go
func openURL(url string) error {
	var err error
	switch runtime.GOOS {
	case "linux":
		err = exec.Command("xdg-open", url).Start()
	case "windows":
		err = exec.Command("rundll32", "url.dll,FileProtocolHandler", url).Start()
	case "darwin":
		err = exec.Command("open", url).Start()
	default:
		err = fmt.Errorf("Cannot open URL %s on this platform", url)
	}
	return err
}

// tokenFromFile retrieves a Token from a given file path.
// It returns the retrieved Token and any read error encountered.
func tokenFromFile(fname string) (*oauth2.Token, error) {
	t := new(oauth2.Token)
	if f, err := os.Open(fname); err != nil {
		return nil, fmt.Errorf("can't open '%s':%w", fname, err)
	} else if err := json.NewDecoder(f).Decode(t); err != nil {
		f.Close()
		return nil, fmt.Errorf("can't unmarshal '%s':%w", fname, err)
	} else {
		f.Close()
		return t, nil
	}
}

// saveToken uses a file path to create a file and store the
// token in it.
func saveToken(fname string, token *oauth2.Token) error {
	if f, err := os.Create(fname); err != nil {
		return fmt.Errorf("can't write '%s':%w", fname, err)
	} else if err := json.NewEncoder(f).Encode(token); err != nil {
		f.Close()
		return fmt.Errorf("can't marshal to '%s':%w", fname, err)
	} else {
		return f.Close()
	}
}

type Youtube struct {
	Client   *http.Client
	Service  *youtube.Service
	CodeChan chan string
}

func New(ctx context.Context) (*Youtube, error) {
	y := new(Youtube)
	return y, nil
}

func (y *Youtube) InitStage1(ctx context.Context) error {
	if c, err := y.getClient(ctx, youtube.YoutubeReadonlyScope); err != nil {
		return fmt.Errorf("can't y.getClient(): %w", err)
	} else {
		y.Client = c
	}

	if s, err := youtube.NewService(ctx, option.WithHTTPClient(y.Client)); err != nil {
		return fmt.Errorf("can't youtube.NewService(): %w", err)
	} else {
		y.Service = s
	}
	return nil
}

func (y *Youtube) Ready() bool {
	return y.Service != nil
}

func (y *Youtube) GetCodeChan() chan string {
	return y.CodeChan
}

const tokenCacheFname = "youtube-go.json"

func (y *Youtube) getClient(ctx context.Context, scope string) (*http.Client, error) {
	app := app2.FromContext(ctx)
	s := []byte(`{"installed":{"client_id":"456702871445-uhsk61jqld6hes2dqiht0jk1gfl8j9mt.apps.googleusercontent.com","project_id":"controlrake","auth_uri":"https://accounts.google.com/o/oauth2/auth","token_uri":"https://oauth2.googleapis.com/token","auth_provider_x509_cert_url":"https://www.googleapis.com/oauth2/v1/certs","client_secret":"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","redirect_uris":["http://localhost"]}}`)

	config := MUST2(google.ConfigFromJSON(s, scope))

	config.RedirectURL = app.HTTP.GetBaseUrl("localhost") + "googleoauth2"

	tok, err := tokenFromFile(tokenCacheFname)
	if err != nil {
		authURL := config.AuthCodeURL("state-token", oauth2.AccessTypeOffline)
		app.Log.Info().Msg("taking youtube token from web")
		if tok, err = y.getTokenFromWeb(ctx, config, authURL); err != nil {
			return nil, fmt.Errorf("getTockenFromWeb(): %w", err)
		}
		saveToken(tokenCacheFname, tok)
	}
	return config.Client(ctx, tok), nil
}

func (y *Youtube) getTokenFromWeb(ctx context.Context, config *oauth2.Config, authURL string) (*oauth2.Token, error) {
	app := app2.FromContext(ctx)
	if err := openURL(authURL); err != nil {
		return nil, fmt.Errorf("Unable to open authorization URL in web server: %w", err)
	} else {
		app.Log.Info().Str("authURL", authURL).Msg("Your browser has been opened to an authorization URL." +
			" This program will resume once authorization has been provided.")
	}

	y.CodeChan = make(chan string)

	// Wait for the web server to get the code.
	code := <-y.CodeChan
	close(y.CodeChan)
	y.CodeChan = nil

	if tok, err := config.Exchange(ctx, code); err != nil {
		return nil, fmt.Errorf("unable to retrieve token %w", err)
	} else {
		return tok, nil
	}
}

func (y *Youtube) GetStreamInfo(ctx context.Context) (*youtube.LiveBroadcast, error) {
	if ret, err := y.Service.LiveBroadcasts.List(nil).BroadcastStatus("active").Context(ctx).Do(); err != nil {
		return nil, fmt.Errorf("can't LiveBroadcasts.List(nil): %w", err)
	} else if len(ret.Items) == 0 {
		return nil, fmt.Errorf("no live broadcast now")
	} else {
		return ret.Items[0], nil
	}
}

type ChatConnection struct {
	PerPage              int
	Yt                   *Youtube
	Info                 *youtube.LiveBroadcast
	PenultimatePageToken string
	PenultimateMesages   []string
	NextPageToken        string
}

func (y *Youtube) GetChatConnection(ctx context.Context, perpage int) *ChatConnection {
	app := app2.FromContext(ctx)
	for {
		if ctx.Err() != nil {
			return nil
		} else if info, err := y.GetStreamInfo(ctx); err != nil {
			app.Log.Error().Err(err).Msg("y.GetStreamInfo(ctx)")
			<-time.After(5 * time.Second)
		} else {
			return &ChatConnection{
				PerPage: perpage,
				Info:    info,
				Yt:      y,
			}
		}
	}

}

// TODO do not retrieve penultimate page if we have messages from it
func (cc *ChatConnection) Spin(ctx context.Context) []string {
	app := app2.FromContext(ctx)

	if info, err := cc.Yt.GetStreamInfo(ctx); err != nil {
		app.Log.Error().Err(err).Msg("y.GetStreamInfo(ctx)")
	} else {
		cc.Info = info
	}

	messages := []string{}
	pt := cc.PenultimatePageToken

	for {
		app.Logger()
		app.Log.Error().Msg("Requesting")
		<-time.After(50 * time.Millisecond)
		req := cc.Yt.Service.LiveChatMessages.
			List(cc.Info.Snippet.LiveChatId, []string{"snippet", "authorDetails"}).
			MaxResults(10).
			ProfileImageSize(16).
			Context(ctx)
		if pt != "" {
			req.PageToken(pt)
		}

		// achtung nextpagetoken will be allways presented
		if ret, err := req.Do(); err != nil {
			app.Log.Error().Err(err).Msg("can't LiveChatMessages.List()")
			<-time.After(5 * time.Second)
			continue

		} else if len(ret.Items) >= cc.PerPage {
			messages = append(messages, LiveChatMessage2Messages(ret.Items)...)
			cc.PenultimatePageToken = pt
			pt = ret.NextPageToken

		} else { // last unfinished page
			messages = append(messages, LiveChatMessage2Messages(ret.Items)...)
			break
		}
	}

	return messages
}

func LiveChatMessage2Messages(in []*youtube.LiveChatMessage) []string {
	n := time.Now()
	return MAP(in, func(inn *youtube.LiveChatMessage) string {
		d, _ := time.Parse(time.RFC3339, inn.Snippet.PublishedAt)
		return fmt.Sprintf("%d %s: %s", n.Sub(d)/time.Second, inn.AuthorDetails.DisplayName, inn.Snippet.DisplayMessage)
	})
}
