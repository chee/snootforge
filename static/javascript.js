let main = jQuery( "main" )

jQuery( document.body ).on( "click", "a", function handle (event) {
	let anchor = jQuery( this )
	let url = anchor.attr( "href" )

	if (url.startsWith( "mailto:" )) {
		console.log( "not doing anything with a mailto:" )
		return
	}

	main.load( `${url} main` )

	event.preventDefault()
})
