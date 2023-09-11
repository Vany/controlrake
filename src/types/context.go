package types

import "context"

var Key = struct{}{}

type Context struct {
	Cfg *Config
}

func WithValues(ctx context.Context, cfg *Config) context.Context {
	return context.WithValue(ctx, Key, &Context{
		Cfg: cfg,
	})
}

func FromContext(ctx context.Context) (cfg *Config, cc *Context) {
	c := ctx.Value(Key) // or die
	if c == nil {
		return nil, nil
	}
	return c.(*Context).Cfg, c.(*Context)
}
