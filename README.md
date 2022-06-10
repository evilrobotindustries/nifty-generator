# Nifty Generator

Nifty Generator (`ng`) is a command line tool for randomly generating images/video/metadata based on a configuration file.

NOTE: this is still pre-release and as such, things are subject to change. 

## Generation
   
The `generate` command will expect to find a `config.json` in the specified source directory, which configures how the various atributes/media elements are to be combined. A sample configuration file can be found at `config.template.json` in the source code above. The resulting output will be generated within the `output` subdirectory by default.

Basic usage:
    
    ng generate /path/to/source/directory

A full listing of all available options can be found using:

    ng generate --help

##  Exploration
Once generated, you can use [Nifty Gallery](https://niftygallery.evilrobot.industries) to explore the generated collection within a browser. This will require two steps:

#### Step 1
Serve the local metadata/media output with a local web server such as [Static Web Server](https://sws.joseluisq.net), sample command as below. The `--root` option should specify the `output` folder where the content was generated. 

    static-web-server --log-level debug --cache-control-headers false --directory-listing true --cors-allow-origins --root /path/to/output --port 8787

You will then be able to browse your content via http://localhost:8787, assuming you used the same port as above. More information at https://sws.joseluisq.net/configuration/command-line-arguments/

#### Step 2
Enter the URL to the metadata of the first token (e.g. http://localhost:8787/metadata/1) into the input box and then browse through the collection.

## Deployment

The `deploy` command allows you to update the generated metadata to point to wherever the media files are hosted. Simply provide the `base-uri` as a command line option.

Basic usage:
    
    ng deploy /path/to/source/directory --base-uri ipfs://SomEIpFSHash/ 
    
A full listing of all available options can be found using:

    ng deploy --help
