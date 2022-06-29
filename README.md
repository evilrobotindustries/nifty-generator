# Nifty Generator

Nifty Generator (`ng`) is a command line tool for randomly generating images/video/metadata based on a configuration file.

NOTE: this is still pre-release and as such, things are subject to change. You will also need to install [FFmpeg](https://ffmpeg.org) and ensure it is within your PATH if you want video generation.

## Generation
   
The `generate` command will expect to find a `config.json` in the specified source directory, which configures how the various atributes/media elements are to be combined. The resulting output will be generated within the `output` subdirectory by default.

Basic usage:
    
    ng generate /path/to/source/directory

A full listing of all available options can be found using:

    ng generate --help
    
### Configuration

The `config.json` structure has the below fields. A sample configuration file can be found at `config.template.json` in the source code above:

| Name | Type | Optional | Description |
| ---- | ---- | -------- | ----------- |
| name | `String` | No | A name for the token, including {id} which will be replaced by the token number. |
| description | `String` | No | A description of the token/project. |
| background_color | `String` | Yes | A optional background color in rgba hex format (e.g. #112233 or #112233FF). |
| external_url | `String` | Yes | An optional url for the token, including {id} which will be replaced by the token number. |
| supply | `Number` | No | The total number of tokens to be generated. |
| start_token | `Number` | No | The number of the first token. |
| attributes | `Array` | No | An array of attributes, which should be specified in layered order: i.e. layer 0 of the image as the bottom/last attribute. |

#### Attribute

An attribute has the following fields:

| Name | Type | Optional | Description |
| ---- | ---- | -------- | ----------- |
| name | `String` | No | The name of the attribute, as it should appear in the resulting token metadata. |
| metadata | `Boolean` | Yes | Whether the attribute should be included in the resulting token metadata (default is `true`). |
| options | `Map` | No | The possible values for the attribute. |

#### Attribute Option

Finally, an attribute option can be of the following types:

##### Audio
Audio files (aac, flac, m4a, mp3, wav) are combined with images to create video.

| Name | Type | Optional | Description |
| ---- | ---- | -------- | ----------- |
| file | `String` | No | The path to the audio file to be used. Supported types are .aac, .flac, .m4a, .mp3, .wav. |
| weight | `Number` | Yes | The weight which determines how frequently the options is included during generation. A smaller weight value increases rarity and when not specified, defaults to 1 and will be ignored if set to zero. |

##### Color
A layer can simply be filled with a color (in rgba hex).

| Name | Type | Optional | Description |
| ---- | ---- | -------- | ----------- |
| color | `String` | No | A color in rgba hex format (e.g. #112233 or #112233FF). |
| weight | `Number` | Yes | As above. |

##### Image
Images can be combined in layers to produce a final generated image.

| Name | Type | Optional | Description |
| ---- | ---- | -------- | ----------- |
| file | `String` | No | The path to the image file to be used. Supported types are .avif, .jpg, .jpeg, .png, .gif, .webp, .tif, .tiff, .tga, .dds, .bmp, .ico, .hdr, .exr, .pbm, .pam, .ppm, .pgm, .ff, farbfeld)  |
| weight | `Number` | Yes | As above. |

##### None
A none/empty option can be added by specifying a weight value only.

| Name | Type | Optional | Description |
| ---- | ---- | -------- | ----------- |
| weight | `Number` | No | As above. |

##### Text
Text can be written to the image using the specified font, color, pixel height and co-ordinates. 

| Name | Type | Optional | Description |
| ---- | ---- | -------- | ----------- |
| font | `String` | No | The path to the font file to be used. |
| text | `String` | No | The text to be included. Any use of {id} will be replaced by the token number. |
| height | `Number` | No | The height of the text, in pixels. |
| x | `Number` | No | The x co-ordinate of where the text should start (in pixels). Use a negative value to right-align the text. |
| y | `Number` | No | The y co-ordinate of where the text should start (in pixels). |
| color | `String` | No | The color of the text in rgba hex format (e.g. #112233 or #112233FF). |
| weight | `Number` | Yes | As above. |


##  Exploration
Once generated, you can use [Nifty Gallery](https://github.com/evilrobotindustries/nifty-gallery) to explore the generated collection within a browser. This will require two steps:

#### Step 1
Serve the local metadata/media output with a local web server such as [Static Web Server](https://sws.joseluisq.net), sample command as below. The `--root` option should specify the `output` folder where the content was generated. 

    static-web-server --log-level debug --cache-control-headers false --directory-listing true --cors-allow-origins "*" --root ./output --port 8787


You will then be able to browse your content via http://localhost:8787, assuming you used the same port as above. More information at https://sws.joseluisq.net/configuration/command-line-arguments/

#### Step 2
Enter the URL to the metadata of the first token (e.g. http://localhost:8787/metadata/1) into the input box and then browse through the collection.

## Deployment

The `deploy` command allows you to update the generated metadata to point to wherever the media files are hosted. Simply provide the `base-uri` as a command line option.

Basic usage:
    
    ng deploy /path/to/source/directory --base-uri ipfs://SomEIpFSHash/ 
    
A full listing of all available options can be found using:

    ng deploy --help
