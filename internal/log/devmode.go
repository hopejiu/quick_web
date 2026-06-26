//go:build !production

package log

func init() {
	DevMode = true
}
